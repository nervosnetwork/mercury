# Migration instruction

## 0.2.2 Release

No migration matters.

## 0.2.1 Release

No migration matters.

## 0.2.0 Release

Starting from 0.2.0, any address in the return value of mercury such as `SignatureAction` and `RecordID` will be encoded as new format full address. Meanwhile, the short address and old format full address will be deprecated.

### From earlier than v0.2.0-beta.4

All versions, except for v0.2.0-beta.4, need to clear the database and resynchronize the data.

### From v0.2.0-beta.4

Upgrading from v0.2.0-beta.4 version do not need to resynchronize data. However, it should be noted that the `extra_filter` field of `Record` strcuture add a new `Freezed` type.

If you are using [SDK](https://github.com/nervosnetwork/mercury#sdk-support) to connect to mercury, you only need to upgrade the SDK version to v0.101.1, otherwise you need to adapt the `Record` structure.
