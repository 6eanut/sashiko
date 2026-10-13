# ATA / libata Subsystem Context

## Log Directory and Quirk Handling (`drivers/ata/libata-core.c`)

- `ata_dev_configure()` and `ata_read_log_directory()` probe device capabilities during initialization and error-handler (`libata-eh.c`) revalidation.
- Setting `dev->quirks |= ATA_QUIRK_NO_LOG_DIR` and calling `ata_clear_log_directory(dev)` permanently suppresses General Purpose Log probing for that device across subsequent reconfigurations.
- Static device quirks belong in `__ata_dev_quirks` (`drivers/ata/libata-core.c`); runtime callers must only disable log probing when I/O or structural corruption prevents safe parsing.
