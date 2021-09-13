............gen macro sql:
 pub async fn
fetch_consume_cell_by_txs(conn : & mut RBatisConnExecutor < '_ >, tx_hashes :
                          Vec < BsonBytes >,) -> rbatis :: core :: Result <
Vec < ConsumedCell > >
{
    let mut rb_args = vec! [] ;
    rb_args.push(bson :: to_bson(& tx_hashes).unwrap()) ; { } use rbatis ::
    executor :: { Executor, ExecutorMut } ; return
    conn.fetch(&
               "SELECT mercury_cell.id, mercury_cell.tx_hash, mercury_cell.output_index, mercury_cell.tx_index, 
    mercury_cell.block_number, mercury_cell.block_hash, mercury_cell.epoch_number, mercury_cell.epoch_index,
    mercury_cell.epoch_length, mercury_cell.capacity, mercury_cell.lock_hash, mercury_cell.lock_code_hash, 
    mercury_cell.lock_args, mercury_cell.lock_script_type, mercury_cell.type_hash, mercury_cell.type_code_hash, 
    mercury_cell.type_args, mercury_cell.type_script_type, mercury_cell.data, mercury_consume_info.consumed_block_number,
    mercury_consume_info.consumed_block_hash, mercury_consume_info.consumed_tx_hash, mercury_consume_info.consumed_tx_index,
    mercury_consume_info.input_index, mercury_consume_info.since
    FROM mercury_cell INNER JOIN mercury_consume_info
    On mercury_cell.tx_hash = mercury_consume_info.tx_hash AND mercury_cell.output_index = mercury_consume_info.output_index
    WHERE mercury_consume_info.consumed_tx_hash IN ($1::bytea)",
               & rb_args).await ;
}
............gen macro sql end............
............gen macro sql:
 pub async fn
is_live_cell(conn : & mut RBatisConnExecutor < '_ >, tx_hash : BsonBytes,
             index : u16,) -> rbatis :: core :: Result < Option < i64 > >
{
    let mut rb_args = vec! [] ;
    rb_args.push(bson :: to_bson(& tx_hash).unwrap()) ;
    rb_args.push(bson :: to_bson(& index).unwrap()) ; { } use rbatis ::
    executor :: { Executor, ExecutorMut } ; return
    conn.fetch(&
               "SELECT id FROM mercury_live_cell WHERE tx_hash = $1::bytea AND output_index = $2",
               & rb_args).await ;
}
............gen macro sql end............
............gen macro sql:
 pub async fn
remove_live_cell(conn : & mut RBatisConnExecutor < '_ >, tx_hash : BsonBytes,
                 index : u16,) -> rbatis :: core :: Result < () >
{
    let mut rb_args = vec! [] ;
    rb_args.push(bson :: to_bson(& tx_hash).unwrap()) ;
    rb_args.push(bson :: to_bson(& index).unwrap()) ; { } use rbatis ::
    executor :: { Executor, ExecutorMut } ; return
    conn.fetch(&
               "DELETE FROM mercury_live_cell WHERE tx_hash = $1::bytea AND output_index = $2",
               & rb_args).await ;
}
............gen macro sql end............
............gen macro sql:
 pub async fn
get_tx_hash_by_block_hash(tx : & mut RBatisTxExecutor < '_ >, block_hash :
                          BsonBytes,) -> rbatis :: core :: Result < Option <
Vec < BsonBytes > > >
{
    let mut rb_args = vec! [] ;
    rb_args.push(bson :: to_bson(& block_hash).unwrap()) ; { } use rbatis ::
    executor :: { Executor, ExecutorMut } ; return
    tx.fetch(&
             "SELECT tx_hash FROM mercury_transaction WHERE tx_hash = $1::bytea",
             & rb_args).await ;
}
............gen macro sql end............
............gen macro sql:
 pub async fn
query_scripts_by_partial_arg(conn : & mut RBatisConnExecutor < '_ >, code_hash
                             : BsonBytes, arg : BsonBytes, from : u32, to :
                             u32,) -> rbatis :: core :: Result < Option < Vec
< ScriptTable > > >
{
    let mut rb_args = vec! [] ;
    rb_args.push(bson :: to_bson(& code_hash).unwrap()) ;
    rb_args.push(bson :: to_bson(& arg).unwrap()) ;
    rb_args.push(bson :: to_bson(& from).unwrap()) ;
    rb_args.push(bson :: to_bson(& to).unwrap()) ; { } use rbatis :: executor
    :: { Executor, ExecutorMut } ; return
    conn.fetch(&
               "SELECT * FROM mercury_script WHERE script_code_hash = $1::bytea IN (SELECT script_code_hash FROM mercury_script WHERE substring(script_args::bytea from $3 for $4) = $2)",
               & rb_args).await ;
}
............gen macro sql end............
............gen macro sql:
 pub async fn
query_current_sync_number(tx : & mut RBatisTxExecutor < '_ >, block_range :
                          u32,) -> rbatis :: core :: Result < Option < u32 > >
{
    let mut rb_args = vec! [] ;
    rb_args.push(bson :: to_bson(& block_range).unwrap()) ; { } use rbatis ::
    executor :: { Executor, ExecutorMut } ; return
    tx.fetch(&
             "SELECT current_sync_number FROM mercury_sync_status WHERE block_range = $1",
             & rb_args).await ;
}
............gen macro sql end............
............gen macro sql:
 pub async fn
update_sync_dead_cell(tx : & mut RBatisTxExecutor < '_ >, tx_hash : BsonBytes,
                      index : u32,) -> rbatis :: core :: Result < () >
{
    let mut rb_args = vec! [] ;
    rb_args.push(bson :: to_bson(& tx_hash).unwrap()) ;
    rb_args.push(bson :: to_bson(& index).unwrap()) ; { } use rbatis ::
    executor :: { Executor, ExecutorMut } ; return
    tx.fetch(&
             "UPDATE mercury_sync_dead_cell SET is_delete = true WHERE tx_hash = $1::bytea and output_index = $2",
             & rb_args).await ;
}
............gen macro sql end............
............gen macro sql:
 pub async fn
fetch_consume_cell_by_txs_sqlite(conn : & mut RBatisConnExecutor < '_ >,
                                 tx_hashes : Vec < BsonBytes >,) -> rbatis ::
core :: Result < Vec < ConsumedCell > >
{
    let mut rb_args = vec! [] ;
    rb_args.push(bson :: to_bson(& tx_hashes).unwrap()) ; { } use rbatis ::
    executor :: { Executor, ExecutorMut } ; return
    conn.fetch(&
               "SELECT mercury_cell.id, mercury_cell.tx_hash, mercury_cell.output_index, mercury_cell.tx_index, 
    mercury_cell.block_number, mercury_cell.block_hash, mercury_cell.epoch_number, mercury_cell.epoch_index,
    mercury_cell.epoch_length, mercury_cell.capacity, mercury_cell.lock_hash, mercury_cell.lock_code_hash, 
    mercury_cell.lock_args, mercury_cell.lock_script_type, mercury_cell.type_hash, mercury_cell.type_code_hash, 
    mercury_cell.type_args, mercury_cell.type_script_type, mercury_cell.data, mercury_consume_info.consumed_block_number,
    mercury_consume_info.consumed_block_hash, mercury_consume_info.consumed_tx_hash, mercury_consume_info.consumed_tx_index,
    mercury_consume_info.input_index, mercury_consume_info.since
    FROM mercury_cell INNER JOIN mercury_consume_info
    On mercury_cell.tx_hash = mercury_consume_info.tx_hash AND mercury_cell.output_index = mercury_consume_info.output_index
    WHERE mercury_consume_info.consumed_tx_hash IN ($1)",
               & rb_args).await ;
}
............gen macro sql end............
............gen impl CRUDTable:
 #[derive(Serialize, Deserialize, Clone, Debug)] pub struct BlockTable
{
    pub block_hash : BsonBytes, pub block_number : u64, pub version : u16, pub
    compact_target : u32, pub block_timestamp : u64, pub epoch_number : u32,
    pub epoch_index : u32, pub epoch_length : u32, pub parent_hash :
    BsonBytes, pub transactions_root : BsonBytes, pub proposals_hash :
    BsonBytes, pub uncles_hash : BsonBytes, pub dao : BsonBytes, pub nonce :
    BsonBytes, pub proposals : BsonBytes,
} impl rbatis :: crud :: CRUDTable for BlockTable
{
    fn get(& self, column : & str) -> serde_json :: Value
    {
        return match column
        {
            "block_hash" =>
            { return serde_json :: json! (& self.block_hash) ; }
            "block_number" =>
            { return serde_json :: json! (& self.block_number) ; } "version"
            => { return serde_json :: json! (& self.version) ; }
            "compact_target" =>
            { return serde_json :: json! (& self.compact_target) ; }
            "block_timestamp" =>
            { return serde_json :: json! (& self.block_timestamp) ; }
            "epoch_number" =>
            { return serde_json :: json! (& self.epoch_number) ; }
            "epoch_index" =>
            { return serde_json :: json! (& self.epoch_index) ; }
            "epoch_length" =>
            { return serde_json :: json! (& self.epoch_length) ; }
            "parent_hash" =>
            { return serde_json :: json! (& self.parent_hash) ; }
            "transactions_root" =>
            { return serde_json :: json! (& self.transactions_root) ; }
            "proposals_hash" =>
            { return serde_json :: json! (& self.proposals_hash) ; }
            "uncles_hash" =>
            { return serde_json :: json! (& self.uncles_hash) ; } "dao" =>
            { return serde_json :: json! (& self.dao) ; } "nonce" =>
            { return serde_json :: json! (& self.nonce) ; } "proposals" =>
            { return serde_json :: json! (& self.proposals) ; } _ =>
            { serde_json :: Value :: Null }
        }
    } fn table_name() -> String { "mercury_block".to_string() } fn
    table_columns() -> String
    {
        "block_hash,block_number,version,compact_target,block_timestamp,epoch_number,epoch_index,epoch_length,parent_hash,transactions_root,proposals_hash,uncles_hash,dao,nonce,proposals".to_string()
    } fn formats(driver_type : & rbatis :: core :: db :: DriverType) -> std ::
    collections :: HashMap < String, fn(arg : & str) -> String >
    {
        let mut m : std :: collections :: HashMap < String, fn(arg : & str) ->
        String > = std :: collections :: HashMap :: new() ; match driver_type
        {
            rbatis :: core :: db :: DriverType :: Mysql => { return m ; },
            rbatis :: core :: db :: DriverType :: Postgres =>
            {
                m.insert("block_hash".to_string(), | arg : & str | -> String
                         { format! ("{}::bytea", arg) }) ;
                m.insert("parent_hash".to_string(), | arg : & str | -> String
                         { format! ("{}::bytea", arg) }) ;
                m.insert("transactions_root".to_string(), | arg : & str | ->
                         String { format! ("{}::bytea", arg) }) ;
                m.insert("proposals_hash".to_string(), | arg : & str | ->
                         String { format! ("{}::bytea", arg) }) ;
                m.insert("uncles_hash".to_string(), | arg : & str | -> String
                         { format! ("{}::bytea", arg) }) ;
                m.insert("dao".to_string(), | arg : & str | -> String
                         { format! ("{}::bytea", arg) }) ;
                m.insert("nonce".to_string(), | arg : & str | -> String
                         { format! ("{}::bytea", arg) }) ;
                m.insert("proposals".to_string(), | arg : & str | -> String
                         { format! ("{}::bytea", arg) }) ; return m ;
            }, rbatis :: core :: db :: DriverType :: Sqlite => { return m ; },
            rbatis :: core :: db :: DriverType :: Mssql => { return m ; },
            rbatis :: core :: db :: DriverType :: None => { return m ; },
        }
    }
}
............gen impl CRUDTable end............
............gen impl CRUDTable:
 #[derive(Serialize, Deserialize, Clone, Debug)] pub struct TransactionTable
{
    pub id : i64, pub tx_hash : BsonBytes, pub tx_index : u32, pub input_count
    : u32, pub output_count : u32, pub block_number : u64, pub block_hash :
    BsonBytes, pub tx_timestamp : u64, pub version : u16, pub cell_deps :
    BsonBytes, pub header_deps : BsonBytes, pub witnesses : BsonBytes,
} impl rbatis :: crud :: CRUDTable for TransactionTable
{
    fn get(& self, column : & str) -> serde_json :: Value
    {
        return match column
        {
            "id" => { return serde_json :: json! (& self.id) ; } "tx_hash" =>
            { return serde_json :: json! (& self.tx_hash) ; } "tx_index" =>
            { return serde_json :: json! (& self.tx_index) ; } "input_count"
            => { return serde_json :: json! (& self.input_count) ; }
            "output_count" =>
            { return serde_json :: json! (& self.output_count) ; }
            "block_number" =>
            { return serde_json :: json! (& self.block_number) ; }
            "block_hash" =>
            { return serde_json :: json! (& self.block_hash) ; }
            "tx_timestamp" =>
            { return serde_json :: json! (& self.tx_timestamp) ; } "version"
            => { return serde_json :: json! (& self.version) ; } "cell_deps"
            => { return serde_json :: json! (& self.cell_deps) ; }
            "header_deps" =>
            { return serde_json :: json! (& self.header_deps) ; } "witnesses"
            => { return serde_json :: json! (& self.witnesses) ; } _ =>
            { serde_json :: Value :: Null }
        }
    } fn table_name() -> String { "mercury_transaction".to_string() } fn
    table_columns() -> String
    {
        "id,tx_hash,tx_index,input_count,output_count,block_number,block_hash,tx_timestamp,version,cell_deps,header_deps,witnesses".to_string()
    } fn formats(driver_type : & rbatis :: core :: db :: DriverType) -> std ::
    collections :: HashMap < String, fn(arg : & str) -> String >
    {
        let mut m : std :: collections :: HashMap < String, fn(arg : & str) ->
        String > = std :: collections :: HashMap :: new() ; match driver_type
        {
            rbatis :: core :: db :: DriverType :: Mysql => { return m ; },
            rbatis :: core :: db :: DriverType :: Postgres =>
            {
                m.insert("tx_hash".to_string(), | arg : & str | -> String
                         { format! ("{}::bytea", arg) }) ;
                m.insert("block_hash".to_string(), | arg : & str | -> String
                         { format! ("{}::bytea", arg) }) ;
                m.insert("cell_deps".to_string(), | arg : & str | -> String
                         { format! ("{}::bytea", arg) }) ;
                m.insert("header_deps".to_string(), | arg : & str | -> String
                         { format! ("{}::bytea", arg) }) ;
                m.insert("witnesses".to_string(), | arg : & str | -> String
                         { format! ("{}::bytea", arg) }) ; return m ;
            }, rbatis :: core :: db :: DriverType :: Sqlite => { return m ; },
            rbatis :: core :: db :: DriverType :: Mssql => { return m ; },
            rbatis :: core :: db :: DriverType :: None => { return m ; },
        }
    }
}
............gen impl CRUDTable end............
............gen impl CRUDTable:
 #[derive(Serialize, Deserialize, Clone, Debug)] pub struct CellTable
{
    pub id : i64, pub tx_hash : BsonBytes, pub output_index : u32, pub
    tx_index : u32, pub block_number : u64, pub block_hash : BsonBytes, pub
    epoch_number : u32, pub epoch_index : u32, pub epoch_length : u32, pub
    capacity : u64, pub lock_hash : BsonBytes, pub lock_code_hash : BsonBytes,
    pub lock_args : BsonBytes, pub lock_script_type : u8, pub type_hash :
    BsonBytes, pub type_code_hash : BsonBytes, pub type_args : BsonBytes, pub
    type_script_type : u8, pub data : BsonBytes,
} impl rbatis :: crud :: CRUDTable for CellTable
{
    fn get(& self, column : & str) -> serde_json :: Value
    {
        return match column
        {
            "id" => { return serde_json :: json! (& self.id) ; } "tx_hash" =>
            { return serde_json :: json! (& self.tx_hash) ; } "output_index"
            => { return serde_json :: json! (& self.output_index) ; }
            "tx_index" => { return serde_json :: json! (& self.tx_index) ; }
            "block_number" =>
            { return serde_json :: json! (& self.block_number) ; }
            "block_hash" =>
            { return serde_json :: json! (& self.block_hash) ; }
            "epoch_number" =>
            { return serde_json :: json! (& self.epoch_number) ; }
            "epoch_index" =>
            { return serde_json :: json! (& self.epoch_index) ; }
            "epoch_length" =>
            { return serde_json :: json! (& self.epoch_length) ; } "capacity"
            => { return serde_json :: json! (& self.capacity) ; } "lock_hash"
            => { return serde_json :: json! (& self.lock_hash) ; }
            "lock_code_hash" =>
            { return serde_json :: json! (& self.lock_code_hash) ; }
            "lock_args" => { return serde_json :: json! (& self.lock_args) ; }
            "lock_script_type" =>
            { return serde_json :: json! (& self.lock_script_type) ; }
            "type_hash" => { return serde_json :: json! (& self.type_hash) ; }
            "type_code_hash" =>
            { return serde_json :: json! (& self.type_code_hash) ; }
            "type_args" => { return serde_json :: json! (& self.type_args) ; }
            "type_script_type" =>
            { return serde_json :: json! (& self.type_script_type) ; } "data"
            => { return serde_json :: json! (& self.data) ; } _ =>
            { serde_json :: Value :: Null }
        }
    } fn table_name() -> String { "mercury_cell".to_string() } fn
    table_columns() -> String
    {
        "id,tx_hash,output_index,tx_index,block_number,block_hash,epoch_number,epoch_index,epoch_length,capacity,lock_hash,lock_code_hash,lock_args,lock_script_type,type_hash,type_code_hash,type_args,type_script_type,data".to_string()
    } fn formats(driver_type : & rbatis :: core :: db :: DriverType) -> std ::
    collections :: HashMap < String, fn(arg : & str) -> String >
    {
        let mut m : std :: collections :: HashMap < String, fn(arg : & str) ->
        String > = std :: collections :: HashMap :: new() ; match driver_type
        {
            rbatis :: core :: db :: DriverType :: Mysql => { return m ; },
            rbatis :: core :: db :: DriverType :: Postgres =>
            {
                m.insert("tx_hash".to_string(), | arg : & str | -> String
                         { format! ("{}::bytea", arg) }) ;
                m.insert("block_hash".to_string(), | arg : & str | -> String
                         { format! ("{}::bytea", arg) }) ;
                m.insert("lock_hash".to_string(), | arg : & str | -> String
                         { format! ("{}::bytea", arg) }) ;
                m.insert("lock_code_hash".to_string(), | arg : & str | ->
                         String { format! ("{}::bytea", arg) }) ;
                m.insert("lock_args".to_string(), | arg : & str | -> String
                         { format! ("{}::bytea", arg) }) ;
                m.insert("type_hash".to_string(), | arg : & str | -> String
                         { format! ("{}::bytea", arg) }) ;
                m.insert("type_code_hash".to_string(), | arg : & str | ->
                         String { format! ("{}::bytea", arg) }) ;
                m.insert("type_args".to_string(), | arg : & str | -> String
                         { format! ("{}::bytea", arg) }) ;
                m.insert("type_script_type".to_string(), | arg : & str | ->
                         String { format! ("{}::smallint", arg) }) ;
                m.insert("data".to_string(), | arg : & str | -> String
                         { format! ("{}::bytea", arg) }) ;
                m.insert("consumed_block_hash".to_string(), | arg : & str | ->
                         String { format! ("{}::bytea", arg) }) ;
                m.insert("consumed_tx_hash".to_string(), | arg : & str | ->
                         String { format! ("{}::bytea", arg) }) ;
                m.insert("since".to_string(), | arg : & str | -> String
                         { format! ("{}::bytea", arg) }) ; return m ;
            }, rbatis :: core :: db :: DriverType :: Sqlite => { return m ; },
            rbatis :: core :: db :: DriverType :: Mssql => { return m ; },
            rbatis :: core :: db :: DriverType :: None => { return m ; },
        }
    }
}
............gen impl CRUDTable end............
............gen impl CRUDTable:
 #[derive(Serialize, Deserialize, Clone, Debug)] pub struct ConsumeInfoTable
{
    pub tx_hash : BsonBytes, pub output_index : u32, pub consumed_block_number
    : u64, pub consumed_block_hash : BsonBytes, pub consumed_tx_hash :
    BsonBytes, pub consumed_tx_index : u32, pub input_index : u32, pub since :
    BsonBytes,
} impl rbatis :: crud :: CRUDTable for ConsumeInfoTable
{
    fn get(& self, column : & str) -> serde_json :: Value
    {
        return match column
        {
            "tx_hash" => { return serde_json :: json! (& self.tx_hash) ; }
            "output_index" =>
            { return serde_json :: json! (& self.output_index) ; }
            "consumed_block_number" =>
            { return serde_json :: json! (& self.consumed_block_number) ; }
            "consumed_block_hash" =>
            { return serde_json :: json! (& self.consumed_block_hash) ; }
            "consumed_tx_hash" =>
            { return serde_json :: json! (& self.consumed_tx_hash) ; }
            "consumed_tx_index" =>
            { return serde_json :: json! (& self.consumed_tx_index) ; }
            "input_index" =>
            { return serde_json :: json! (& self.input_index) ; } "since" =>
            { return serde_json :: json! (& self.since) ; } _ =>
            { serde_json :: Value :: Null }
        }
    } fn table_name() -> String { "mercury_consume_info".to_string() } fn
    table_columns() -> String
    {
        "tx_hash,output_index,consumed_block_number,consumed_block_hash,consumed_tx_hash,consumed_tx_index,input_index,since".to_string()
    } fn formats(driver_type : & rbatis :: core :: db :: DriverType) -> std ::
    collections :: HashMap < String, fn(arg : & str) -> String >
    {
        let mut m : std :: collections :: HashMap < String, fn(arg : & str) ->
        String > = std :: collections :: HashMap :: new() ; match driver_type
        {
            rbatis :: core :: db :: DriverType :: Mysql => { return m ; },
            rbatis :: core :: db :: DriverType :: Postgres =>
            {
                m.insert("tx_hash".to_string(), | arg : & str | -> String
                         { format! ("{}::bytea", arg) }) ;
                m.insert("consumed_block_hash".to_string(), | arg : & str | ->
                         String { format! ("{}::bytea", arg) }) ;
                m.insert("consumed_tx_hash".to_string(), | arg : & str | ->
                         String { format! ("{}::bytea", arg) }) ;
                m.insert("since".to_string(), | arg : & str | -> String
                         { format! ("{}::bytea", arg) }) ; return m ;
            }, rbatis :: core :: db :: DriverType :: Sqlite => { return m ; },
            rbatis :: core :: db :: DriverType :: Mssql => { return m ; },
            rbatis :: core :: db :: DriverType :: None => { return m ; },
        }
    }
}
............gen impl CRUDTable end............
............gen impl CRUDTable:
 #[derive(Serialize, Deserialize, Clone, Debug)] pub struct LiveCellTable
{
    pub id : i64, pub tx_hash : BsonBytes, pub output_index : u32, pub
    tx_index : u32, pub block_number : u64, pub block_hash : BsonBytes, pub
    epoch_number : u32, pub epoch_index : u32, pub epoch_length : u32, pub
    capacity : u64, pub lock_hash : BsonBytes, pub lock_code_hash : BsonBytes,
    pub lock_args : BsonBytes, pub lock_script_type : u8, pub type_hash :
    BsonBytes, pub type_code_hash : BsonBytes, pub type_args : BsonBytes, pub
    type_script_type : u8, pub data : BsonBytes,
} impl rbatis :: crud :: CRUDTable for LiveCellTable
{
    fn get(& self, column : & str) -> serde_json :: Value
    {
        return match column
        {
            "id" => { return serde_json :: json! (& self.id) ; } "tx_hash" =>
            { return serde_json :: json! (& self.tx_hash) ; } "output_index"
            => { return serde_json :: json! (& self.output_index) ; }
            "tx_index" => { return serde_json :: json! (& self.tx_index) ; }
            "block_number" =>
            { return serde_json :: json! (& self.block_number) ; }
            "block_hash" =>
            { return serde_json :: json! (& self.block_hash) ; }
            "epoch_number" =>
            { return serde_json :: json! (& self.epoch_number) ; }
            "epoch_index" =>
            { return serde_json :: json! (& self.epoch_index) ; }
            "epoch_length" =>
            { return serde_json :: json! (& self.epoch_length) ; } "capacity"
            => { return serde_json :: json! (& self.capacity) ; } "lock_hash"
            => { return serde_json :: json! (& self.lock_hash) ; }
            "lock_code_hash" =>
            { return serde_json :: json! (& self.lock_code_hash) ; }
            "lock_args" => { return serde_json :: json! (& self.lock_args) ; }
            "lock_script_type" =>
            { return serde_json :: json! (& self.lock_script_type) ; }
            "type_hash" => { return serde_json :: json! (& self.type_hash) ; }
            "type_code_hash" =>
            { return serde_json :: json! (& self.type_code_hash) ; }
            "type_args" => { return serde_json :: json! (& self.type_args) ; }
            "type_script_type" =>
            { return serde_json :: json! (& self.type_script_type) ; } "data"
            => { return serde_json :: json! (& self.data) ; } _ =>
            { serde_json :: Value :: Null }
        }
    } fn table_name() -> String { "mercury_live_cell".to_string() } fn
    table_columns() -> String
    {
        "id,tx_hash,output_index,tx_index,block_number,block_hash,epoch_number,epoch_index,epoch_length,capacity,lock_hash,lock_code_hash,lock_args,lock_script_type,type_hash,type_code_hash,type_args,type_script_type,data".to_string()
    } fn formats(driver_type : & rbatis :: core :: db :: DriverType) -> std ::
    collections :: HashMap < String, fn(arg : & str) -> String >
    {
        let mut m : std :: collections :: HashMap < String, fn(arg : & str) ->
        String > = std :: collections :: HashMap :: new() ; match driver_type
        {
            rbatis :: core :: db :: DriverType :: Mysql => { return m ; },
            rbatis :: core :: db :: DriverType :: Postgres =>
            {
                m.insert("tx_hash".to_string(), | arg : & str | -> String
                         { format! ("{}::bytea", arg) }) ;
                m.insert("block_hash".to_string(), | arg : & str | -> String
                         { format! ("{}::bytea", arg) }) ;
                m.insert("lock_hash".to_string(), | arg : & str | -> String
                         { format! ("{}::bytea", arg) }) ;
                m.insert("lock_code_hash".to_string(), | arg : & str | ->
                         String { format! ("{}::bytea", arg) }) ;
                m.insert("lock_args".to_string(), | arg : & str | -> String
                         { format! ("{}::bytea", arg) }) ;
                m.insert("type_hash".to_string(), | arg : & str | -> String
                         { format! ("{}::bytea", arg) }) ;
                m.insert("type_code_hash".to_string(), | arg : & str | ->
                         String { format! ("{}::bytea", arg) }) ;
                m.insert("type_args".to_string(), | arg : & str | -> String
                         { format! ("{}::bytea", arg) }) ;
                m.insert("type_script_type".to_string(), | arg : & str | ->
                         String { format! ("{}::int", arg) }) ;
                m.insert("data".to_string(), | arg : & str | -> String
                         { format! ("{}::bytea", arg) }) ; return m ;
            }, rbatis :: core :: db :: DriverType :: Sqlite => { return m ; },
            rbatis :: core :: db :: DriverType :: Mssql => { return m ; },
            rbatis :: core :: db :: DriverType :: None => { return m ; },
        }
    }
}
............gen impl CRUDTable end............
............gen impl CRUDTable:
 #[derive(Serialize, Deserialize, Clone, Debug)] pub struct ScriptTable
{
    pub script_hash : BsonBytes, pub script_hash_160 : BsonBytes, pub
    script_code_hash : BsonBytes, pub script_args : BsonBytes, pub script_type
    : u8, pub script_args_len : u32,
} impl rbatis :: crud :: CRUDTable for ScriptTable
{
    fn get(& self, column : & str) -> serde_json :: Value
    {
        return match column
        {
            "script_hash" =>
            { return serde_json :: json! (& self.script_hash) ; }
            "script_hash_160" =>
            { return serde_json :: json! (& self.script_hash_160) ; }
            "script_code_hash" =>
            { return serde_json :: json! (& self.script_code_hash) ; }
            "script_args" =>
            { return serde_json :: json! (& self.script_args) ; }
            "script_type" =>
            { return serde_json :: json! (& self.script_type) ; }
            "script_args_len" =>
            { return serde_json :: json! (& self.script_args_len) ; } _ =>
            { serde_json :: Value :: Null }
        }
    } fn table_name() -> String { "mercury_script".to_string() } fn
    table_columns() -> String
    {
        "script_hash,script_hash_160,script_code_hash,script_args,script_type,script_args_len".to_string()
    } fn formats(driver_type : & rbatis :: core :: db :: DriverType) -> std ::
    collections :: HashMap < String, fn(arg : & str) -> String >
    {
        let mut m : std :: collections :: HashMap < String, fn(arg : & str) ->
        String > = std :: collections :: HashMap :: new() ; match driver_type
        {
            rbatis :: core :: db :: DriverType :: Mysql => { return m ; },
            rbatis :: core :: db :: DriverType :: Postgres =>
            {
                m.insert("script_hash".to_string(), | arg : & str | -> String
                         { format! ("{}::bytea", arg) }) ;
                m.insert("script_hash_160".to_string(), | arg : & str | ->
                         String { format! ("{}::bytea", arg) }) ;
                m.insert("script_code_hash".to_string(), | arg : & str | ->
                         String { format! ("{}::bytea", arg) }) ;
                m.insert("script_args".to_string(), | arg : & str | -> String
                         { format! ("{}::bytea", arg) }) ; return m ;
            }, rbatis :: core :: db :: DriverType :: Sqlite => { return m ; },
            rbatis :: core :: db :: DriverType :: Mssql => { return m ; },
            rbatis :: core :: db :: DriverType :: None => { return m ; },
        }
    }
}
............gen impl CRUDTable end............
............gen impl CRUDTable:
 #[derive(Serialize, Deserialize, Clone, Debug)] pub struct
UncleRelationshipTable
{ pub block_hash : BsonBytes, pub uncle_hashes : BsonBytes, } impl rbatis ::
crud :: CRUDTable for UncleRelationshipTable
{
    fn get(& self, column : & str) -> serde_json :: Value
    {
        return match column
        {
            "block_hash" =>
            { return serde_json :: json! (& self.block_hash) ; }
            "uncle_hashes" =>
            { return serde_json :: json! (& self.uncle_hashes) ; } _ =>
            { serde_json :: Value :: Null }
        }
    } fn table_name() -> String { "mercury_uncle_relationship".to_string() }
    fn table_columns() -> String { "block_hash,uncle_hashes".to_string() } fn
    formats(driver_type : & rbatis :: core :: db :: DriverType) -> std ::
    collections :: HashMap < String, fn(arg : & str) -> String >
    {
        let mut m : std :: collections :: HashMap < String, fn(arg : & str) ->
        String > = std :: collections :: HashMap :: new() ; match driver_type
        {
            rbatis :: core :: db :: DriverType :: Mysql => { return m ; },
            rbatis :: core :: db :: DriverType :: Postgres =>
            {
                m.insert("block_hash".to_string(), | arg : & str | -> String
                         { format! ("{}::bytea", arg) }) ;
                m.insert("uncle_hashes".to_string(), | arg : & str | -> String
                         { format! ("{}::bytea", arg) }) ; return m ;
            }, rbatis :: core :: db :: DriverType :: Sqlite => { return m ; },
            rbatis :: core :: db :: DriverType :: Mssql => { return m ; },
            rbatis :: core :: db :: DriverType :: None => { return m ; },
        }
    }
}
............gen impl CRUDTable end............
............gen impl CRUDTable:
 #[derive(Serialize, Deserialize, Clone, Debug)] pub struct CanonicalChainTable
{ pub block_number : u64, pub block_hash : BsonBytes, } impl rbatis :: crud ::
CRUDTable for CanonicalChainTable
{
    fn get(& self, column : & str) -> serde_json :: Value
    {
        return match column
        {
            "block_number" =>
            { return serde_json :: json! (& self.block_number) ; }
            "block_hash" =>
            { return serde_json :: json! (& self.block_hash) ; } _ =>
            { serde_json :: Value :: Null }
        }
    } fn table_name() -> String { "mercury_canonical_chain".to_string() } fn
    table_columns() -> String { "block_number,block_hash".to_string() } fn
    formats(driver_type : & rbatis :: core :: db :: DriverType) -> std ::
    collections :: HashMap < String, fn(arg : & str) -> String >
    {
        let mut m : std :: collections :: HashMap < String, fn(arg : & str) ->
        String > = std :: collections :: HashMap :: new() ; match driver_type
        {
            rbatis :: core :: db :: DriverType :: Mysql => { return m ; },
            rbatis :: core :: db :: DriverType :: Postgres =>
            {
                m.insert("block_hash".to_string(), | arg : & str | -> String
                         { format! ("{}::bytea", arg) }) ; return m ;
            }, rbatis :: core :: db :: DriverType :: Sqlite => { return m ; },
            rbatis :: core :: db :: DriverType :: Mssql => { return m ; },
            rbatis :: core :: db :: DriverType :: None => { return m ; },
        }
    }
}
............gen impl CRUDTable end............
............gen impl CRUDTable:
 #[derive(Serialize, Deserialize, Clone, Debug)] pub struct
RegisteredAddressTable { pub lock_hash : BsonBytes, pub address : String, }
impl rbatis :: crud :: CRUDTable for RegisteredAddressTable
{
    fn get(& self, column : & str) -> serde_json :: Value
    {
        return match column
        {
            "lock_hash" => { return serde_json :: json! (& self.lock_hash) ; }
            "address" => { return serde_json :: json! (& self.address) ; } _
            => { serde_json :: Value :: Null }
        }
    } fn table_name() -> String { "mercury_registered_address".to_string() }
    fn table_columns() -> String { "lock_hash,address".to_string() } fn
    formats(driver_type : & rbatis :: core :: db :: DriverType) -> std ::
    collections :: HashMap < String, fn(arg : & str) -> String >
    {
        let mut m : std :: collections :: HashMap < String, fn(arg : & str) ->
        String > = std :: collections :: HashMap :: new() ; match driver_type
        {
            rbatis :: core :: db :: DriverType :: Mysql => { return m ; },
            rbatis :: core :: db :: DriverType :: Postgres =>
            {
                m.insert("lock_hash".to_string(), | arg : & str | -> String
                         { format! ("{}::bytea", arg) }) ; return m ;
            }, rbatis :: core :: db :: DriverType :: Sqlite => { return m ; },
            rbatis :: core :: db :: DriverType :: Mssql => { return m ; },
            rbatis :: core :: db :: DriverType :: None => { return m ; },
        }
    }
}
............gen impl CRUDTable end............

running 1 test
test relational::tests::get_cell_test::test_get_consumed_cell ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 15 filtered out; finished in 0.04s

