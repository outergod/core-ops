variant: fcos
version: 1.6.0

passwd:
  users:
    - name: core
      ssh_authorized_keys:
        - ${SSH_PUBLIC_KEY}

storage:
  files:
    - path: /etc/NetworkManager/system-connections/static.nmconnection
      mode: 0600
      contents:
        inline: |
          [connection]
          id=static
          type=ethernet
          interface-name=eth0

          [ipv4]
          method=manual
          addresses=${STATIC_IP}/24
          gateway=192.168.1.1
          dns=192.168.1.4

          [ipv6]
          method=ignore
