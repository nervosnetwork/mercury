use core::panic;


pub fn post_http_request(body: &'static str) -> serde_json::Value {
    let client = reqwest::blocking::Client::new();
    let resp = client.post("http://127.0.0.1:8116")
        .header("content-type", "application/json")
        .body(body)
        .send().unwrap();
    if ! resp.status().is_success() {
        panic!("Not 200 Status Code. [status_code={}]", resp.status());
    }

    let text = resp.text().unwrap();
    println!("[request]:\n{}", body);
    println!("[response]:\n{}\n", text);

    serde_json::from_str(&text).unwrap()
}
