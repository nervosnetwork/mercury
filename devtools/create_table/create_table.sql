CREATE TABLE block(
    block_hash varchar(64) PRIMARY KEY,
    block_number bigint NOT NULL,
    version int NOT NULL,
    compact_target int NOT NULL,
    epoch bigint NOT NULL,
    parent_hash varchar(64) NOT NULL,
    transaction_root varchar(64) NOT NULL,
    proposals_hash varchar(64) NOT NULL,
    uncles_hash varchar(64),
    dao varchar(64) NOT NULL,
    nonce bigint NOT NULL,
    proposals varchar
);

CREATE TABLE transaction(
    id bigserial PRIMARY KEY,
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
    id bigserial PRIMARY KEY,
    output_index int NOT NULL,
    tx_hash varchar(64) NOT NULL,
    tx_index int NOT NULL,
    block_hash varchar(64) NOT NULL,
    block_number bigint NOT NULL,
    epoch_number bigint NOT NULL,
    capacity bigint NOT NULL,
    lock_hash varchar(64) NOT NULL,
    lock_code_hash varchar(64) NOT NULL,
    lock_script_hash varchar(64) NOT NULL,
    lock_args varchar NOT NULL,
    lock_script_type int NOT NULL,
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
    since bigint
);

CREATE TABLE live_cell(
    id bigserial PRIMARY KEY,
    output_index int NOT NULL,
    tx_hash varchar(64) NOT NULL,
    tx_index int NOT NULL,
    block_hash varchar(64) NOT NULL,
    block_number bigint NOT NULL,
    epoch_number bigint NOT NULL,
    capacity bigint NOT NULL,
    lock_hash varchar(64) NOT NULL,
    lock_code_hash varchar(64) NOT NULL,
    lock_script_hash varchar(64) NOT NULL,
    lock_args varchar NOT NULL,
    lock_script_type int NOT NULL,
    type_hash varchar(64),
    type_code_hash varchar(64),
    type_args varchar,
    type_script_type int,
    data varchar,
    id_data_complete bool
);

CREATE TABLE script(
    id bigserial PRIMARY KEY,
    script_hash varchar(64) NOT NULL,
    script_hash_160 varchar(50) NOT NULL,
    script_code_hash varchar(64) NOT NULL,
    script_args varchar,
    script_type int NOT NULL,
    args_len int
);

CREATE TABLE big_data(
    tx_hash varchar(64),
    output_index varchar(64),
    data varchar NOT NULL,
    PRIMARY KEY(tx_hash, output_index)
);

CREATE TABLE uncle_relationship(
    block_hash varchar(64),
    uncle_hash varchar(64),
    PRIMARY KEY(block_hash, uncle_hash)
);

CREATE TABLE canonical_chain(
    block_number bigint PRIMARY KEY,
    block_hash varchar(64) NOT NULL
);
