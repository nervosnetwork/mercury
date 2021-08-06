CREATE TABLE block(
    block_hash varchar(64) PRIMARY KEY,
    block_number bigint NOT NULL,
    version int NOT NULL,
    compact_target int NOT NULL,
    timestamp bigint NOT NULL,
    epoch bigint NOT NULL,
    parent_hash varchar(64) NOT NULL,
    transactions_root varchar(64) NOT NULL,
    proposals_hash varchar(64) NOT NULL,
    uncles_hash varchar,
    dao varchar(64) NOT NULL,
    nonce varchar NOT NULL,
    proposals varchar
);

CREATE TABLE transaction(
    id bigint PRIMARY KEY,
    tx_hash varchar(64) NOT NULL,
    tx_index int NOT NULL,
    input_count int NOT NULL,
    output_count int NOT NULL,
    block_number bigint NOT NULL,
    block_hash varchar(64) NOT NULL,
    timestamp bigint NOT NULL,
    version int NOT NULL,
    cell_deps varchar,
    header_deps varchar,
    witnesses varchar
);

CREATE TABLE cell(
    id bigint PRIMARY KEY,
    tx_hash varchar(64) NOT NULL,
    output_index int NOT NULL,
    tx_index int NOT NULL,
    block_hash varchar(64) NOT NULL,
    block_number bigint NOT NULL,
    epoch_number bigint NOT NULL,
    capacity bigint NOT NULL,
    lock_hash varchar(64),
    lock_code_hash varchar(64),
    lock_args varchar,
    lock_script_type int,
    type_hash varchar(64),
    type_code_hash varchar(64),
    type_args varchar,
    type_script_type int,
    data varchar,
    is_data_complete bool,
    consumed_block_number bigint,
    consumed_block_hash varchar(64),
    consumed_tx_hash varchar(64),
    consumed_tx_index int,
    input_index int,
    since bigint
);

CREATE TABLE live_cell(
    id bigint PRIMARY KEY,
    output_index int NOT NULL,
    tx_hash varchar(64) NOT NULL,
    tx_index int NOT NULL,
    block_hash varchar(64) NOT NULL,
    block_number bigint NOT NULL,
    epoch_number bigint NOT NULL,
    capacity bigint NOT NULL,
    lock_hash varchar(64),
    lock_code_hash varchar(64),
    lock_script_hash varchar(64),
    lock_args varchar,
    lock_script_type int,
    type_hash varchar(64),
    type_code_hash varchar(64),
    type_args varchar,
    type_script_type int,
    data varchar,
    is_data_complete bool
);

CREATE TABLE script(
    id bigint PRIMARY KEY,
    script_hash varchar(64) NOT NULL,
    script_hash_160 varchar(40) NOT NULL,
    script_code_hash varchar(64) NOT NULL,
    script_args varchar,
    script_type int NOT NULL,
    script_args_len int
);

CREATE TABLE big_data(
    tx_hash varchar(64),
    output_index varchar(64),
    data varchar NOT NULL,
    PRIMARY KEY(tx_hash, output_index)
);

CREATE TABLE uncle_relationship(
    block_hash varchar(64),
    uncles_hash varchar,
    PRIMARY KEY(block_hash, uncles_hash)
);

CREATE TABLE canonical_chain(
    block_number bigint PRIMARY KEY,
    block_hash varchar(64) NOT NULL
);
