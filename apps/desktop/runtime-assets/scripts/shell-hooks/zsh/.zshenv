if [[ "${(t)DMUX_ORIGINAL_ZSHENV_SOURCED:-}" == *export* ]]; then
  unset DMUX_ORIGINAL_ZSHENV_SOURCED
fi

_dmux_source_user_startup_file() {
  local startup_file="$1"
  local user_zdotdir="$2"
  local runtime_zdotdir="$3"
  local restore_process_launched_by_q=0
  local restore_q_term=0

  if [[ -z "${PROCESS_LAUNCHED_BY_Q+x}" ]]; then
    export PROCESS_LAUNCHED_BY_Q=wecode
    restore_process_launched_by_q=1
  fi
  if [[ -z "${Q_TERM+x}" ]]; then
    export Q_TERM=wecode
    restore_q_term=1
  fi

  export ZDOTDIR="${user_zdotdir}"
  source "${startup_file}"
  export ZDOTDIR="${runtime_zdotdir}"

  if (( restore_process_launched_by_q )) && [[ "${PROCESS_LAUNCHED_BY_Q:-}" == wecode ]]; then
    unset PROCESS_LAUNCHED_BY_Q
  fi
  if (( restore_q_term )) && [[ "${Q_TERM:-}" == wecode ]]; then
    unset Q_TERM
  fi
}

if [[ -z "${DMUX_ORIGINAL_ZSHENV_SOURCED:-}" ]]; then
  typeset -g DMUX_ORIGINAL_ZSHENV_SOURCED=1
  dmux_user_zdotdir="${DMUX_USER_ZDOTDIR:-${HOME}}"
  dmux_runtime_zdotdir="${ZDOTDIR:-}"
  if [[ -f "${dmux_user_zdotdir}/.zshenv" ]]; then
    _dmux_source_user_startup_file "${dmux_user_zdotdir}/.zshenv" "${dmux_user_zdotdir}" "${dmux_runtime_zdotdir}"
  fi
  unset dmux_user_zdotdir dmux_runtime_zdotdir
fi
