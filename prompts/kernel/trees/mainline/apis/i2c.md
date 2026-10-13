# I2C Transfer and DMA Bounce Buffer APIs (`include/linux/i2c.h`)

## Client Transfer Buffer DMA Rules

* **Ordinary Transfers (`i2c_transfer`, `i2c_master_send`, `i2c_master_recv`, `i2c_smbus_*`)**:
  * Safe to call with stack-allocated, struct-embedded, or vmalloc buffers.
  * Do **not** flag stack buffers passed to ordinary I2C transfer APIs as DMA bugs; when an adapter uses DMA, the I2C core bounce-buffers any message that lacks `I2C_M_DMA_SAFE`.
* **DMA-Safe Transfers (`i2c_master_send_dmasafe`, `i2c_master_recv_dmasafe`, `I2C_M_DMA_SAFE`)**:
  * Assert that the buffer is already DMA-safe (`kmalloc`/`kzalloc`, cacheline-aligned, not on the stack or in vmalloc).
  * Passing a stack, vmalloc, or non-cacheline-aligned buffer to `*_dmasafe()` or setting `I2C_M_DMA_SAFE` on it bypasses bounce buffering and is a bug.

## Adapter DMA Bounce Helpers (`drivers/i2c/i2c-core-base.c`)

* I2C controller (`struct i2c_algorithm`) `master_xfer` implementations that perform DMA must call `i2c_get_dma_safe_msg_buf(msg, threshold)` before mapping a message buffer for DMA (`NULL` return indicates the driver should fall back to PIO).
* On transaction completion or failure, the adapter driver must call `i2c_put_dma_safe_msg_buf(buf, msg, xferred)` across all paths to copy received data back and free any temporary bounce buffer.
