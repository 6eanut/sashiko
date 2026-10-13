# I2C Subsystem Code Context (`drivers/i2c/i2c-core-base.c`)

## Client `debugfs` Lifecycle

* Debugfs entries created under `client->debugfs` (`struct i2c_client`) are automatically removed by the I2C core during device unbind (after the driver's `.remove()` callback returns and before devres release) and after `.probe()` failure. Drivers do not need to manually remove entries under `client->debugfs`.
