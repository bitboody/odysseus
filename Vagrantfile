# -*- mode: ruby -*-
# vi: set ft=ruby :

# Secure Windows sandbox for running the Odysseus Tauri launcher.
# ================================================================
# The .exe runs entirely inside the VM. NOTHING is written back to your host.
#
# Quick start:
#   vagrant up
#   vagrant snapshot save clean
#   vagrant rdp
#     login: OdysseusUser
#     pass:  Odysseus123!
#   # double-click "Odysseus" on the desktop and use it normally
#
# Reset after use:
#   vagrant snapshot restore clean
# ================================================================

Vagrant.configure("2") do |config|
  config.vm.box = "peru/windows-10-enterprise-x64-eval"
  config.vm.box_check_update = true
  config.vm.hostname = "odysseus-sandbox"

  config.vm.communicator = "winrm"
  config.winrm.username  = "vagrant"
  config.winrm.password  = "vagrant"
  config.winrm.retry_limit = 30
  config.winrm.retry_delay = 10

  # No synced folders to the host project directory.
  # This guarantees the guest CANNOT write anything to your host filesystem.
  config.vm.synced_folder ".", "/vagrant", disabled: true

  # RDP forward (Vagrant auto-corrects the host port if 3389 is taken)
  config.vm.network "forwarded_port", guest: 3389, host: 3389, id: "rdp", auto_correct: true

  config.vm.provider "virtualbox" do |vb|
    vb.name = "odysseus-sandbox"
    vb.cpus = 4
    vb.memory = "8192"
    # Set to true if you want a native VM window (useful if RDP is unavailable)
    vb.gui = false

    # Video RAM required for WebView2 rendering
    vb.customize ["modifyvm", :id, "--vram", "128"]

    # Isolate host <-> guest channels
    vb.customize ["modifyvm", :id, "--clipboard-mode", "disabled"]
    vb.customize ["modifyvm", :id, "--drag-and-drop", "disabled"]
    vb.customize ["modifyvm", :id, "--usbehci", "off"]
    vb.customize ["modifyvm", :id, "--usbxhci", "off"]

    # Enable nested VT-x only if your CPU supports it.
    # Needed only if you later install Docker Desktop inside the VM
    # so the launcher’s full installer path (git clone + docker-compose) works.
    vb.customize ["modifyvm", :id, "--nested-hw-virt", "on"]
  end

  # -----------------------------------------------------------------
  # Provisioning
  # -----------------------------------------------------------------
  # 1) Create a non-admin user and harden the VM
  config.vm.provision "isolation",
    type: "shell",
    path: "vagrant/provision-isolation.ps1",
    privileged: true

  # 2) Build or fetch the .exe and place it on the user's desktop
  config.vm.provision "build",
    type: "shell",
    path: "vagrant/provision-build.ps1",
    privileged: true

  # -----------------------------------------------------------------
  # OPTIONAL: skip the build and copy a pre-built .exe from the host
  # -----------------------------------------------------------------
  # Uncomment the block below if you already have odysseus.exe on your
  # host (e.g., in the same folder as this Vagrantfile) and want to
  # copy it into the VM instead of compiling from source.
  #
  # config.vm.provision "upload-exe",
  #   type: "file",
  #   source: "odysseus.exe",
  #   destination: "C:/OdysseusBuild/odysseus.exe"
  #
  # config.vm.provision "stage-desktop",
  #   type: "shell",
  #   inline: <<-SHELL
  #     $user = "OdysseusUser"
  #     $desktop = "C:\Users\$user\Desktop"
  #     New-Item -ItemType Directory -Force -Path $desktop | Out-Null
  #     Copy-Item "C:\OdysseusBuild\odysseus.exe" "$desktop\odysseus.exe" -Force
  #     $Wsh = New-Object -ComObject WScript.Shell
  #     $lnk = $Wsh.CreateShortcut("$desktop\Odysseus.lnk")
  #     $lnk.TargetPath = "C:\OdysseusBuild\odysseus.exe"
  #     $lnk.WorkingDirectory = "C:\OdysseusBuild"
  #     $lnk.IconLocation = "C:\OdysseusBuild\odysseus.exe,0"
  #     $lnk.Save()
  #     # grant read/execute
  #     $acl = Get-Acl "C:\OdysseusBuild"
  #     $rule = New-Object System.Security.AccessControl.FileSystemAccessRule($user, "ReadAndExecute", "ContainerInherit,ObjectInherit", "None", "Allow")
  #     $acl.SetAccessRule($rule)
  #     Set-Acl "C:\OdysseusBuild" $acl
  #   SHELL,
  #   privileged: true
end
