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
    consumed_block_number int,
    consumed_block_hash bytea,
    consumed_tx_hash bytea,
    consumed_tx_index int,
    input_index int,
    since bigint
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
    lock_script_hash bytea,
    lock_args bytea,
    lock_script_type smallint,
    type_hash bytea,
    type_code_hash bytea,
    type_args bytea,
    type_script_type smallint,
    data bytea
);

CREATE TABLE mercury_script(
    id bigint PRIMARY KEY,
    script_hash bytea NOT NULL,
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

CREATE TABLE mercury_sync_dead_cell(
    tx_hash bytea NOT NULL,
    output_index int NOT NULL,
    is_delete bool NOT NULL,
    PRIMARY KEY(tx_hash, output_index)
);
