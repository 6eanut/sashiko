# I2C Subsystem Maintainer Notes

## Device ID Tables (`struct i2c_device_id`)

* Initialized arrays of type `struct i2c_device_id` must be declared `const`.
* New or modified entries should use named initializers (`.name = "...", .driver_data = ...`); existing positional entries in untouched rows are not bugs.
