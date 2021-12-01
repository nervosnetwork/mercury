# Migration instruction

## 0.2.0 Release

### From earlier than v0.2.0-beta.4

All versions except v0.2.0-beta.4 need to clear the database and resynchronize all data.

### From v0.2.0-beta.4

Upgrading from v0.2.0-beta.4 version do not need to resynchronize data. However, it should be noted that the `extra_filter` field of `Record` strcuture has been changed:

* The semantics has been changed.

* Add a new type `Freezed`.

If you are using [SDK](https://github.com/nervosnetwork/mercury#sdk-support) to connect to mercury, you only need to upgrade the SDK version, otherwise you need to adapt the `Record` structure.
