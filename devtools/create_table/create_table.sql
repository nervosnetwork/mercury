CREATE TABLE block(
    block_hash bytea PRIMARY KEY,
    block_number bigint NOT NULL,
    version int NOT NULL,
    compact_target int NOT NULL,
    block_timestamp bigint NOT NULL,
    epoch bigint NOT NULL,
    parent_hash bytea NOT NULL,
    transactions_root varchar NOT NULL,
    proposals_hash varchar NOT NULL,
    uncles_hash varchar,
    dao varchar NOT NULL,
    nonce varchar NOT NULL,
    proposals varchar
);

CREATE TABLE transaction(
    id bigint PRIMARY KEY,
    tx_hash bytea NOT NULL,
    tx_index int NOT NULL,
    input_count int NOT NULL,
    output_count int NOT NULL,
    block_number bigint NOT NULL,
    block_hash bytea NOT NULL,
    tx_timestamp bigint NOT NULL,
    version int NOT NULL,
    cell_deps bytea,
    header_deps bytea,
    witnesses bytea
);

CREATE TABLE cell(
    id bigint PRIMARY KEY,
    tx_hash bytea NOT NULL,
    output_index int NOT NULL,
    tx_index int NOT NULL,
    block_hash bytea NOT NULL,
    block_number bigint NOT NULL,
    epoch_number bigint NOT NULL,
    capacity bigint NOT NULL,
    lock_hash bytea,
    lock_code_hash bytea,
    lock_args bytea,
    lock_script_type int,
    type_hash bytea,
    type_code_hash bytea,
    type_args bytea,
    type_script_type int,
    data bytea,
    is_data_complete bool,
    consumed_block_number bigint,
    consumed_block_hash bytea,
    consumed_tx_hash bytea,
    consumed_tx_index int,
    input_index int,
    since bigint
);

CREATE TABLE live_cell(
    id bigint PRIMARY KEY,
    output_index int NOT NULL,
    tx_hash bytea NOT NULL,
    tx_index int NOT NULL,
    block_hash bytea NOT NULL,
    block_number bigint NOT NULL,
    epoch_number bigint NOT NULL,
    capacity bigint NOT NULL,
    lock_hash bytea,
    lock_code_hash bytea,
    lock_script_hash bytea,
    lock_args bytea,
    lock_script_type int,
    type_hash bytea,
    type_code_hash bytea,
    type_args bytea,
    type_script_type int,
    data bytea,
    is_data_complete bool
);

CREATE TABLE script(
    id bigint PRIMARY KEY,
    script_hash bytea NOT NULL,
    script_hash_160 bytea NOT NULL,
    script_code_hash bytea NOT NULL,
    script_args bytea,
    script_type int NOT NULL,
    script_args_len int
);

CREATE TABLE big_data(
    tx_hash bytea,
    output_index int,
    data bytea NOT NULL,
    PRIMARY KEY(tx_hash, output_index)
);

CREATE TABLE uncle_relationship(
    block_hash bytea,
    uncles_hash varchar,
    PRIMARY KEY(block_hash, uncles_hash)
);

CREATE TABLE canonical_chain(
    block_number bigint PRIMARY KEY,
    block_hash bytea NOT NULL
);
