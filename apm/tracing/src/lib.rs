use arc_swap::ArcSwap;
use minitrace::{span::Span, Collector};
use minitrace_jaeger::Reporter;
use tokio::sync::mpsc::{unbounded_channel, UnboundedSender};

use std::net::SocketAddr;
use std::sync::Arc;

lazy_static::lazy_static! {
    pub static ref TRACING_SPAN_TX: ArcSwap<UnboundedSender<Vec<Span>>> = {
        let (tx, _) = unbounded_channel();
        ArcSwap::from_pointee(tx)
    };
}

pub fn init_jaeger(jaeger_uri: String) {
    let (tx, mut rx) = unbounded_channel();
    TRACING_SPAN_TX.swap(Arc::new(tx));
    let uri = jaeger_uri.parse::<SocketAddr>().unwrap();

    tokio::spawn(async move {
        loop {
            if let Some(spans) = rx.recv().await {
                if !spans.is_empty() {
                    let s = spans.get(0).cloned().unwrap();
                    let bytes = Reporter::encode(
                        s.event.to_string(),
                        s.id.into(),
                        s.parent_id.into(),
                        0,
                        &spans,
                    )
                    .unwrap();
                    Reporter::report(uri, &bytes).ok();
                }
            }
        }
    });
}

pub struct MercuryTrace {
    collector: Option<Collector>,
    tx: Arc<UnboundedSender<Vec<Span>>>,
}

impl Default for MercuryTrace {
    fn default() -> Self {
        MercuryTrace {
            collector: None,
            tx: (*TRACING_SPAN_TX.load()).clone(),
        }
    }
}

impl MercuryTrace {
    pub fn new(collector: Collector) -> Self {
        MercuryTrace {
            collector: Some(collector),
            tx: (*TRACING_SPAN_TX.load()).clone(),
        }
    }
}

impl Drop for MercuryTrace {
    fn drop(&mut self) {
        if let Some(collector) = self.collector.take() {
            let spans = collector.collect();
            let _ = self.tx.send(spans);
        }
    }
}
