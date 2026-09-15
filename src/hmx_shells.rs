use anyhow::{Result, bail};

pub fn generate(shell: &str) -> Result<String> {
    match shell {
        "bash" => Ok(bash()),
        "fish" => Ok(fish()),
        "zsh" => Ok(zsh()),
        _ => bail!("Unsupported shell '{}'. Expected bash, fish, or zsh", shell),
    }
}

fn bash() -> String {
    r#"# Bash completions for hmx
_hmx() {
  local cur prev
  COMPREPLY=()
  cur="${COMP_WORDS[COMP_CWORD]}"
  prev="${COMP_WORDS[COMP_CWORD-1]}"
  if [[ "$prev" == --config || "$prev" == -c ]]; then
    COMPREPLY=($(compgen -f -- "$cur")); return
  fi
  local command_index=1
  while (( command_index < COMP_CWORD )); do
    case "${COMP_WORDS[command_index]}" in
      --config|-c|--remote|--session) ((command_index+=2)) ;;
      -*) ((command_index+=1)) ;;
      *) break ;;
    esac
  done
  if [[ "${COMP_WORDS[command_index]}" == machines ]]; then
    if [[ "$cur" == -* ]]; then
      COMPREPLY=($(compgen -W "--config --remote --session --verbose --help" -- "$cur"))
    elif (( COMP_CWORD == command_index + 1 )); then
      COMPREPLY=($(compgen -W "sync" -- "$cur"))
    fi
    return
  fi
  case "$prev" in
    open|o)
      local running configured candidates candidate
      running="$(hmx __list-running 2>/dev/null)"
      configured="$(hmx __list-configured 2>/dev/null)"
      candidates="$running"
      for candidate in $configured; do
        if ! grep -qxF "$candidate" <<< "$running"; then candidates="$candidates $candidate"; fi
      done
      COMPREPLY=($(compgen -W "$candidates" -- "$cur")); return ;;
    close|c|refresh|r) COMPREPLY=($(compgen -W "$(hmx __list-running)" -- "$cur")); return ;;
    completions) COMPREPLY=($(compgen -W "bash fish zsh" -- "$cur")); return ;;
  esac
  COMPREPLY=($(compgen -W "open o close c refresh r list ls init validate machines completions --config --remote --session --verbose --help" -- "$cur"))
}
complete -F _hmx hmx
"#.to_string()
}

fn fish() -> String {
    r#"# Fish completions for hmx
complete -c hmx -f
complete -c hmx -s c -l config -r -d 'Path to shared tmx config'
complete -c hmx -l remote -r -d 'SSH target'
complete -c hmx -l session -r -d 'Herdr session'
complete -c hmx -s v -l verbose -d 'Print commands'
complete -c hmx -n '__fish_use_subcommand' -a open -d 'Open or focus a workspace'
complete -c hmx -n '__fish_use_subcommand' -a close -d 'Close a workspace'
complete -c hmx -n '__fish_use_subcommand' -a refresh -d 'Refresh a workspace'
complete -c hmx -n '__fish_use_subcommand' -a list -d 'List workspaces'
complete -c hmx -n '__fish_use_subcommand' -a init
complete -c hmx -n '__fish_use_subcommand' -a validate
complete -c hmx -n '__fish_use_subcommand' -a completions
complete -c hmx -n '__fish_use_subcommand' -a machines -d 'Manage configured Herdr machines'
complete -c hmx -n '__fish_seen_subcommand_from machines; and not __fish_seen_subcommand_from sync' -a sync -d 'Register configured remote sessions alongside Local'
complete -c hmx -n '__fish_use_subcommand' -a o -d 'Alias for open'
complete -c hmx -n '__fish_use_subcommand' -a c -d 'Alias for close'
complete -c hmx -n '__fish_use_subcommand' -a r -d 'Alias for refresh'
complete -c hmx -n '__fish_use_subcommand' -a ls -d 'Alias for list'
function __hmx_open_workspaces
  set -l running (hmx __list-running 2>/dev/null)
  for candidate in $running; echo -e "$candidate\tRunning"; end
  for candidate in (hmx __list-configured 2>/dev/null)
    if not contains -- $candidate $running; echo -e "$candidate\tConfigured"; end
  end
end
complete -c hmx -n '__fish_seen_subcommand_from open o' -a '(__hmx_open_workspaces)'
complete -c hmx -n '__fish_seen_subcommand_from close c refresh r' -a '(hmx __list-running)'
complete -c hmx -n '__fish_seen_subcommand_from completions' -a 'bash fish zsh'
"#
    .to_string()
}

fn zsh() -> String {
    r#"#compdef hmx
_hmx() {
  local -a commands
  commands=('open:Open or focus a workspace' 'o:Alias for open' 'close:Close a workspace' 'c:Alias for close' 'refresh:Refresh a workspace' 'r:Alias for refresh' 'list:List workspaces' 'ls:Alias for list' 'init:Initialize config' 'validate:Validate config' 'machines:Manage configured Herdr machines' 'completions:Generate completions')
  _arguments -C \
    '(-c --config)'{-c,--config}'[shared config]:file:_files' \
    '--remote[SSH target]:target:' \
    '--session[Herdr session]:session:' \
    '(-v --verbose)'{-v,--verbose}'[print commands]' \
    '1:command:->command' '*::argument:->args'
  case $state in
    command) _describe command commands ;;
    args)
      case $words[1] in
        open|o)
          local -a running configured candidates
          running=(${(f)"$(hmx __list-running 2>/dev/null)"})
          configured=(${(f)"$(hmx __list-configured 2>/dev/null)"})
          candidates=($running)
          for candidate in $configured; do (( ! ${running[(Ie)$candidate]} )) && candidates+=($candidate); done
          _values workspace $candidates ;;
        close|c|refresh|r) _values workspace ${(@f)"$(hmx __list-running)"} ;;
        completions) _values shell bash fish zsh ;;
        machines)
          if (( CURRENT == 2 )); then
            local -a machine_commands
            machine_commands=('sync:Register configured remote sessions alongside Local')
            _describe 'machine command' machine_commands
          fi ;;
      esac ;;
  esac
}
_hmx "$@"
"#.to_string()
}
