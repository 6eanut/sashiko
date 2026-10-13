# Media / V4L2 Subdevice Code Context (`drivers/media/v4l2-core/v4l2-subdev.c`)

## V4L2 Subdevice State Validation Wrappers (`check_state` / `check_format`)

* **`v4l2_subdev_state_get_format()` Pre-Validation via `v4l2_subdev_call()`**:
  * `v4l2_subdev_state_get_format(state, pad, stream)` can return `NULL` in general.
  * However, when a `v4l2_subdev_pad_ops` callback (`enum_mbus_code`, `get_fmt`, `set_fmt`, `enum_frame_size`, `enum_frame_interval`, `get_selection`, `set_selection`) is invoked exclusively through the media-core wrappers (`call_enum_mbus_code()`, `call_get_fmt()`, `call_set_fmt()`, etc. in `drivers/media/v4l2-core/v4l2-subdev.c`), `check_state()` validates the exact `(state, pad, stream)` tuple first (including calling `v4l2_subdev_state_get_format(state, pad, stream)` when `V4L2_SUBDEV_FL_STREAMS` is set under `CONFIG_VIDEO_V4L2_SUBDEV_API` and returning `-EINVAL` if `NULL`).
* **When a Missing `NULL` Check Is NOT a Bug**:
  * If every caller of the callback goes through the matching `call_*` wrapper in `v4l2-subdev.c` and the callback passes the exact same `(state, pad, stream)` arguments validated by `check_state()`, do not report a missing `NULL` check on `v4l2_subdev_state_get_format()`, and cite the `call_*` wrapper + `check_state()` as proof.
* **When a Missing `NULL` Check IS a Bug**:
  * Any direct caller (such as `.init_state` or an internal driver call) bypasses the `call_*` wrapper.
  * The callback queries a different pad, stream, or helper such as `v4l2_subdev_state_get_opposite_stream_format()` (which performs a separate routing lookup not validated by `check_state()`).
