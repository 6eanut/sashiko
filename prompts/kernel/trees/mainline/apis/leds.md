# LED Class and Trigger APIs (`include/linux/leds.h`)

## Lifecycle and Unregistration Ordering

* **`led_classdev_register()` / `led_classdev_unregister()` vs `devm_led_classdev_register[_ext]()`**:
  * Unmanaged `led_classdev_register()` exposes sysfs attributes backed by the driver's `struct led_classdev`. In `.remove()` and `.probe()` error paths, `led_classdev_unregister()` **must** be called before freeing the containing driver private structure (`ddata`), otherwise sysfs access races with or follows `kfree(ddata)`.
  * `devm_led_classdev_register()` and `devm_led_classdev_register_ext()` tie unregistration to devres LIFO release order.
* **`led_trigger_register()` / `led_trigger_unregister()` vs `devm_led_trigger_register()`**:
  * Custom LED triggers registered with `led_trigger_register()` are linked into the global `trigger_list` and must be unregistered via `led_trigger_unregister()` on probe failure or module unload (or registered via `devm_led_trigger_register()`).
