render-ignition name keyfile="~/.ssh/id_ed25519.pub":
    SSH_PUBLIC_KEY="$(cat {{keyfile}})" \
      envsubst < infra/ignition/{{name}}.bu.tpl > infra/ignition/{{name}}.bu
    butane infra/ignition/{{name}}.bu -o infra/ignition/{{name}}.ign
