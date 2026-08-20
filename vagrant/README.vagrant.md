# Odysseus Tauri Launcher – Secure Windows Sandbox

A **disposable Windows VM** to build and safely use the Odysseus Tauri desktop launcher.

**Core promise:** the `.exe` is built and runs **inside the VM** only. No synced folder means **nothing is written back to your host computer**.

---

## What this does

| Goal | How |
|------|-----|
| **Build the .exe** | Rust, Tauri CLI, MSVC, WebView2 are installed inside the VM; the source is cloned from GitHub and compiled there. |
| **Run the .exe safely** | You log in as a normal Windows user (not an admin) and use the app normally — double-click the icon, browse, install, whatever. |
| **Protect your host** | No host-folder sharing. Clipboard, drag-and-drop, and USB passthrough are disabled. If the binary misbehaves, the blast radius is the VM. |
| **Reset easily** | One `vagrant snapshot restore clean` wipes the VM back to a pristine state. |

---

## Prerequisites

- [Vagrant](https://developer.hashicorp.com/vagrant/downloads)
- [VirtualBox](https://www.virtualbox.org/wiki/Downloads)
- ~25 GB free disk space
- 8 GB RAM (12 GB recommended)

> Nested VT-x is enabled so you can optionally install Docker Desktop inside the VM later if you want to test the full launcher installer path (`git clone` + `docker-compose`). If your CPU does not support it, comment out `--nested-hw-virt on` in the `Vagrantfile`.

---

## Quick start

```bash
# 1. Create the VM and build the binary (first run downloads the Windows eval image)
vagrant up

# 2. Save a clean snapshot before you start using the app
vagrant snapshot save clean

# 3. RDP into the VM
vagrant rdp
#   Login: OdysseusUser
#   Password: Odysseus123!
#
#   Then double-click the "Odysseus" icon on the Desktop and use the launcher normally.
```

When finished (or if something goes wrong):

```bash
# Reset the VM to the pristine state — all changes the app made are erased
vagrant snapshot restore clean
```

To destroy the VM entirely:

```bash
vagrant destroy -f
```

---

## If you already have a pre-built `.exe`

If you want to skip building and just test an existing executable (e.g., downloaded from a CI artefact):

1. Place the `.exe` in the same folder as this `Vagrantfile`.
2. Uncomment the `file` provisioner block in the `Vagrantfile` (see the commented section at the bottom of the file) and point it to your `.exe`.
3. `vagrant up`
4. The file will be copied into `C:\OdysseusBuild` inside the VM, and a desktop shortcut created automatically.

*(If you do this, nothing in the VM can write back to the folder on your host because the default synced folder is disabled.)*

---

## How it works

### Isolation

The `Vagrantfile` disables the default synced folder and turns off host↔guest channels:

- **Clipboard** — disabled
- **Drag-and-drop** — disabled
- **USB passthrough** — disabled
- **Inbound networking** — no forwarded ports into your host

This means the guest does not have a writable path to your host filesystem and cannot leak data through the usual virtualisation side channels.

### Users

| Account | Role | Password |
|---------|------|----------|
| `vagrant` | Admin (provisioning only) | `vagrant` |
| `OdysseusUser` | Standard user (you use this one) | `Odysseus123!` |

The launcher runs as `OdysseusUser`, so any UAC prompts or privilege-escalation attempts are blocked by Windows unless you explicitly approve them.

### Firewall

- **Inbound:** blocked (the VM is not exposed to your LAN).
- **Outbound:** allowed (so the launcher can reach the internet for updates, etc.).
- **Logging:** blocked packets are logged to `C:\Windows\system32\LogFiles\Firewall\pfirewall.log` inside the VM.

---

## Troubleshooting

| Symptom | Fix |
|---------|-----|
| `vagrant up` hangs | The Windows eval image is large (~6 GB download) and initial boot + Windows Updates can take 20–30 min on first run. |
| WebView2 window is blank / black | Ensure the VM has at least 128 MB video RAM (set in the `Vagrantfile`) and the VirtualBox display adapter is VBoxSVGA or VMSVGA. |
| RDP connection fails | Use `vagrant rdp` — it automatically forwards to the correct host port. If you prefer a manual client, run `vagrant port rdp` to find the mapped port. |
| Build fails with "frontendDist not found" | The build script creates a placeholder `dist/` automatically; if it still fails, verify `tauri.conf.json` points to `../dist`. |
| Need to test Docker Desktop path | Install Docker Desktop inside the VM manually or via Chocolatey (`choco install docker-desktop`). Note: this requires nested VT-x and extra RAM. |

---

## Files

| File | Purpose |
|------|---------|
| `Vagrantfile` | VM definition (VirtualBox, Win10 eval, 4 vCPU / 8 GB, no synced folders, isolated channels) |
| `vagrant/provision-isolation.ps1` | Create standard user, firewall block inbound |
| `vagrant/provision-build.ps1` | Install Chocolatey, Git, Node, MSVC, Rust, WebView2, Tauri CLI; clone & build; stage `.exe` on desktop |
