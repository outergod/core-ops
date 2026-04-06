variant: fcos
version: 1.6.0

passwd:
  users:
    - name: core
      ssh_authorized_keys:
        - ${SSH_PUBLIC_KEY}

storage:
  files:
    - path: /etc/hostname
      mode: 0644
      contents:
        inline: |
          ${VM_HOSTNAME}

systemd:
  units:
    - name: core-ops-init-repo.service
      enabled: true
      contents: |
        [Unit]
        Description=Initialize bare CoreOps repo
        After=network-online.target
        Wants=network-online.target

        [Service]
        Type=oneshot
        RemainAfterExit=yes
        ExecStart=/usr/bin/bash -lc 'mkdir -p /var/lib/core-ops/repo && chown core: /var/lib/core-ops/repo && test -d /var/lib/core-ops/repo/objects || su -s /usr/bin/bash -c "git init --bare /var/lib/core-ops/repo" core'

        [Install]
        WantedBy=multi-user.target
