# PCI Subsystem Maintainer Notes (`drivers/pci/`, `include/linux/pci.h`)

## Modern IRQ Vector API vs Legacy MSI

- **Do not use legacy `pci_enable_msi()` / `pci_disable_msi()` in new code:** `pci_enable_msi()` and `pci_disable_msi()` are legacy interfaces (`drivers/pci/msi/api.c`) that lack MSI-X support. New PCI driver code must use `pci_alloc_irq_vectors()` and `pci_free_irq_vectors()`.

## Bus-Owned Device Naming

- **Never rename `&pdev->dev`:** PCI drivers must not call `dev_set_name()` on their parent PCI device (`&pdev->dev`) or any other bus-owned `struct device`, as the PCI core owns PCI device naming in sysfs. Only newly allocated child `struct device` instances owned by the driver may be named with `dev_set_name()`.
