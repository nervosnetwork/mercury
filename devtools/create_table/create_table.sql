CREATE TABLE block(
    block_hash bytea PRIMARY KEY,
    block_number int NOT NULL,
    version smallint NOT NULL,
    compact_target int NOT NULL,
    block_timestamp bigint NOT NULL,
    epoch bigint NOT NULL,
    parent_hash bytea NOT NULL,
    transactions_root bytea NOT NULL,
    proposals_hash bytea NOT NULL,
    uncles_hash bytea,
    dao bytea NOT NULL,
    nonce bytea NOT NULL,
    proposals bytea
);

CREATE TABLE transaction(
    id bigint PRIMARY KEY,
    tx_hash bytea NOT NULL,
    tx_index smallint NOT NULL,
    input_count smallint NOT NULL,
    output_count smallint NOT NULL,
    block_number int NOT NULL,
    block_hash bytea NOT NULL,
    tx_timestamp bigint NOT NULL,
    version smallint NOT NULL,
    cell_deps bytea,
    header_deps bytea,
    witnesses bytea
);

CREATE TABLE cell(
    id bigint PRIMARY KEY,
    tx_hash bytea NOT NULL,
    output_index smallint NOT NULL,
    tx_index smallint NOT NULL,
    block_hash bytea NOT NULL,
    block_number int NOT NULL,
    epoch_number int NOT NULL,
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
    is_data_complete bool,
    consumed_block_number bigint,
    consumed_block_hash bytea,
    consumed_tx_hash bytea,
    consumed_tx_index smallint,
    input_index smallint,
    since bigint
);

CREATE TABLE live_cell(
    id bigint PRIMARY KEY,
    output_index smallint NOT NULL,
    tx_hash bytea NOT NULL,
    tx_index smallint NOT NULL,
    block_hash bytea NOT NULL,
    block_number int NOT NULL,
    epoch_number int NOT NULL,
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
    data bytea,
    is_data_complete bool
);

CREATE TABLE script(
    id bigint PRIMARY KEY,
    script_hash bytea NOT NULL,
    script_hash_160 bytea NOT NULL,
    script_code_hash bytea NOT NULL,
    script_args bytea,
    script_type smallint NOT NULL,
    script_args_len smallint
);

CREATE TABLE big_data(
    tx_hash bytea,
    output_index smallint,
    data bytea NOT NULL,
    PRIMARY KEY(tx_hash, output_index)
);

CREATE TABLE uncle_relationship(
    block_hash bytea,
    uncles_hash bytea,
    PRIMARY KEY(block_hash, uncles_hash)
);

CREATE TABLE canonical_chain(
    block_number bigint PRIMARY KEY,
    block_hash bytea NOT NULL
);
