CREATE TABLE mercury_block(
    block_hash bytea PRIMARY KEY,
    block_number int NOT NULL,
    version smallint NOT NULL,
    compact_target int NOT NULL,
    block_timestamp bigint NOT NULL,
    epoch_number int NOT NULL,
    epoch_index int NOT NULL,
    epoch_length int NOT NULL,
    parent_hash bytea NOT NULL,
    transactions_root bytea NOT NULL,
    proposals_hash bytea NOT NULL,
    uncles_hash bytea,
    uncles bytea,
    uncles_count int,
    dao bytea NOT NULL,
    nonce bytea NOT NULL,
    proposals bytea
);

CREATE TABLE mercury_transaction(
    id bigint PRIMARY KEY,
    tx_hash bytea NOT NULL,
    tx_index int NOT NULL,
    input_count int NOT NULL,
    output_count int NOT NULL,
    block_number int NOT NULL,
    block_hash bytea NOT NULL,
    tx_timestamp bigint NOT NULL,
    version smallint NOT NULL,
    cell_deps bytea,
    header_deps bytea,
    witnesses bytea
);

CREATE TABLE mercury_cell(
    id bigint PRIMARY KEY,
    tx_hash bytea NOT NULL,
    output_index int NOT NULL,
    tx_index int NOT NULL,
    block_hash bytea NOT NULL,
    block_number int NOT NULL,
    epoch_number int NOT NULL,
    epoch_index int NOT NULL,
    epoch_length int NOT NULL,
    capacity bigint NOT NULL,
    lock_hash bytea,
    lock_code_hash bytea,
    lock_args bytea,
    lock_script_type smallint,
    type_hash bytea,
    type_code_hash bytea,
    type_args bytea,
    type_script_type smallint,
    data bytea,
    consumed_block_number bigint,
    consumed_block_hash bytea,
    consumed_tx_hash bytea,
    consumed_tx_index int,
    input_index int,
    since bytea
);

CREATE TABLE mercury_live_cell(
    id bigint PRIMARY KEY,
    tx_hash bytea NOT NULL,
    output_index int NOT NULL,
    tx_index int NOT NULL,
    block_hash bytea NOT NULL,
    block_number int NOT NULL,
    epoch_number int NOT NULL,
    epoch_index int NOT NULL,
    epoch_length int NOT NULL,
    capacity bigint NOT NULL,
    lock_hash bytea,
    lock_code_hash bytea,
    lock_args bytea,
    lock_script_type smallint,
    type_hash bytea,
    type_code_hash bytea,
    type_args bytea,
    type_script_type smallint,
    data bytea
);

CREATE TABLE mercury_indexer_cell(
    id bigint PRIMARY KEY,
    block_number int NOT NULL,
    io_type smallint NOT NULL,
    io_index int NOT NULL,
    tx_hash bytea NOT NULL,
    tx_index int NOT NULL,
    lock_hash bytea,
    lock_code_hash bytea,
    lock_args bytea,
    lock_script_type smallint,
    type_hash bytea,
    type_code_hash bytea,
    type_args bytea,
    type_script_type smallint
);

CREATE TABLE mercury_script(
    script_hash bytea NOT NULL PRIMARY KEY,
    script_hash_160 bytea NOT NULL,
    script_code_hash bytea NOT NULL,
    script_args bytea,
    script_type smallint NOT NULL,
    script_args_len int
);

CREATE TABLE mercury_uncle_relationship(
    block_hash bytea,
    uncle_hashes bytea,
    PRIMARY KEY(block_hash, uncle_hashes)
);

CREATE TABLE mercury_canonical_chain(
    block_number int PRIMARY KEY,
    block_hash bytea NOT NULL
);

CREATE TABLE mercury_registered_address(
    lock_hash bytea NOT NULL PRIMARY KEY,
    address varchar NOT NULL
);

CREATE TABLE mercury_sync_status(
    block_number int NOT NULL PRIMARY KEY
);

CREATE TABLE mercury_in_update(
    is_in bool NOT NULL PRIMARY KEY
);

CREATE INDEX "index_block_table_block_number" ON "mercury_block" ("block_number");

CREATE INDEX "index_live_cell_table_block_hash" ON "mercury_live_cell" ("block_hash");
CREATE INDEX "index_live_cell_table_block_number" ON "mercury_live_cell" ("block_number");
CREATE INDEX "index_live_cell_table_tx_hash_and_output_index" ON "mercury_live_cell" ("tx_hash", "output_index");
CREATE INDEX "index_live_cell_table_lock_hash" ON "mercury_live_cell" ("lock_hash");
CREATE INDEX "index_live_cell_table_type_hash" ON "mercury_live_cell" ("type_hash");
CREATE INDEX "index_live_cell_table_lock_code_hash_and_lock_script_type" ON "mercury_live_cell" ("lock_code_hash", "lock_script_type");
CREATE INDEX "index_live_cell_table_type_code_hash_and_type_script_type" ON "mercury_live_cell" ("type_code_hash", "type_script_type");

CREATE INDEX "index_script_table_script_hash" ON "mercury_script" ("script_hash");
CREATE INDEX "index_script_table_code_hash" ON "mercury_script" ("script_code_hash");
CREATE INDEX "index_script_table_args" ON "mercury_script" ("script_args");

CREATE INDEX "index_cell_table_tx_hash_and_output_index" ON "mercury_cell" ("tx_hash", "output_index");
CREATE INDEX "index_cell_table_lock_hash" ON "public"."mercury_cell" ("lock_hash");
CREATE INDEX "index_cell_table_type_hash" ON "public"."mercury_cell" ("type_hash");
CREATE INDEX "index_cell_table_lock_code_hash_and_lock_script_type" ON "public"."mercury_cell" ("lock_code_hash", "lock_script_type");
CREATE INDEX "index_cell_table_type_code_hash_and_type_script_type" ON "public"."mercury_cell" ("type_code_hash", "type_script_type");
CREATE INDEX "index_cell_table_consume_tx_hash_and_consumed_tx_index" ON "public"."mercury_cell" ("consumed_tx_hash", "consumed_tx_index");
CREATE INDEX "index_cell_table_block_number" ON "public"."mercury_cell" USING btree (
  "block_number" "pg_catalog"."int4_ops" ASC NULLS LAST
);
CREATE INDEX "index_cell_table_consumed_block_number" ON "public"."mercury_cell" USING btree (
  "consumed_block_number" "pg_catalog"."int8_ops" ASC NULLS LAST
);

CREATE INDEX "index_transaction_table_tx_hash" ON "mercury_transaction" USING btree ("tx_hash" "pg_catalog"."bytea_ops" ASC NULLS LAST);
