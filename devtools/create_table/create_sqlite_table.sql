CREATE TABLE mercury_block(
    block_hash blob PRIMARY KEY,
    block_number int NOT NULL,
    version smallint NOT NULL,
    compact_target int NOT NULL,
    block_timestamp bigint NOT NULL,
    epoch_number int NOT NULL,
    epoch_block_index smallint NOT NULL,
    epoch_length smallint NOT NULL,
    parent_hash blob NOT NULL,
    transactions_root blob NOT NULL,
    proposals_hash blob NOT NULL,
    uncles_hash blob,
    dao blob NOT NULL,
    nonce blob NOT NULL,
    proposals blob
);

CREATE TABLE mercury_transaction(
    id bigint PRIMARY KEY,
    tx_hash blob NOT NULL,
    tx_index smallint NOT NULL,
    input_count smallint NOT NULL,
    output_count smallint NOT NULL,
    block_number int NOT NULL,
    block_hash blob NOT NULL,
    tx_timestamp bigint NOT NULL,
    version smallint NOT NULL,
    cell_deps blob,
    header_deps blob,
    witnesses blob
);

CREATE TABLE mercury_cell(
    id bigint PRIMARY KEY,
    tx_hash blob NOT NULL,
    output_index smallint NOT NULL,
    tx_index smallint NOT NULL,
    block_hash blob NOT NULL,
    block_number int NOT NULL,
    epoch_number blob NOT NULL,
    capacity bigint NOT NULL,
    lock_hash blob,
    lock_code_hash blob,
    lock_args blob,
    lock_script_type smallint,
    type_hash blob,
    type_code_hash blob,
    type_args blob,
    type_script_type smallint,
    data blob,
    is_data_complete bool,
    consumed_block_number int,
    consumed_block_hash blob,
    consumed_tx_hash blob,
    consumed_tx_index smallint,
    input_index smallint,
    since bigint
);

CREATE TABLE mercury_live_cell(
    id bigint PRIMARY KEY,
    output_index smallint NOT NULL,
    tx_hash blob NOT NULL,
    tx_index smallint NOT NULL,
    block_hash blob NOT NULL,
    block_number int NOT NULL,
    epoch_number blob NOT NULL,
    capacity bigint NOT NULL,
    lock_hash blob,
    lock_code_hash blob,
    lock_script_hash blob,
    lock_args blob,
    lock_script_type smallint,
    type_hash blob,
    type_code_hash blob,
    type_args blob,
    type_script_type smallint,
    data blob,
    is_data_complete bool
);

CREATE TABLE mercury_script(
    id bigint PRIMARY KEY,
    script_hash blob NOT NULL,
    script_hash_160 blob NOT NULL,
    script_code_hash blob NOT NULL,
    script_args blob,
    script_type smallint NOT NULL,
    script_args_len smallint
);

CREATE TABLE mercury_uncle_relationship(
    block_hash blob,
    uncle_hashes blob,
    PRIMARY KEY(block_hash, uncle_hashes)
);

CREATE TABLE mercury_canonical_chain(
    block_number int PRIMARY KEY,
    block_hash blob NOT NULL
);
