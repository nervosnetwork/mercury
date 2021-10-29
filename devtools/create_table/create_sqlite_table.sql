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
    uncles blob,
    uncles_count int,
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
    epoch_number int NOT NULL,
    epoch_index int NOT NULL,
    epoch_length int NOT NULL,
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
    consumed_block_number bigint,
    consumed_block_hash blob,
    consumed_tx_hash blob,
    consumed_tx_index int,
    input_index int,
    since blob 
);

CREATE TABLE mercury_live_cell(
    id bigint PRIMARY KEY,
    output_index smallint NOT NULL,
    tx_hash blob NOT NULL,
    tx_index smallint NOT NULL,
    block_hash blob NOT NULL,
    block_number int NOT NULL,
    epoch_number int NOT NULL,
    epoch_index int NOT NULL,
    epoch_length int NOT NULL,
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
    data blob
);

CREATE TABLE mercury_indexer_cell(
    id bigint PRIMARY KEY,
    block_number int NOT NULL,
    io_type smallint NOT NULL,
    io_index int NOT NULL,
    tx_hash blob NOT NULL,
    tx_index int NOT NULL,
    lock_hash blob,
    lock_code_hash blob,
    lock_args blob,
    lock_script_type smallint,
    type_hash blob,
    type_code_hash blob,
    type_args blob,
    type_script_type smallint
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

CREATE TABLE mercury_registered_address(
    lock_hash blob NOT NULL PRIMARY KEY,
    address varchar NOT NULL
);

CREATE TABLE mercury_consume_info(
    tx_hash blob NOT NULL,
    output_index int NOT NULL,
    consumed_block_number bigint NOT NULL,
    consumed_block_hash blob NOT NULL,
    consumed_tx_hash blob NOT NULL,
    consumed_tx_index int NOT NULL,
    input_index int NOT NULL,
    since blob NOT NULL,
    PRIMARY KEY(tx_hash, output_index)
);
