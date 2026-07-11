#!/usr/bin/env bash
# Copyright 2026 Nathan M. Fraske
# Licensed under the Apache License, Version 2.0; see LICENSE.
#
# UserPromptSubmit forcing hook: substrate-first. The owner's standing problem is
# forgetfulness: an agent answers "do we have X yet" from memory, or authors a value
# from the ether, when the durable in-repo source of truth already holds the answer.
# This hook reads the incoming prompt and, when it reads as a "is X built" question or
# as value-authoring intent, INJECTS a forced reminder to consult the source of truth
# BEFORE answering. Injection (stdout on exit 0) is added to the model's context; it
# does not block, so a false positive costs one extra reminder line, never a stall.
set -u
input="$(cat)"
prompt="$(printf '%s' "$input" | python3 -c 'import json,sys; print(json.load(sys.stdin).get("prompt",""))' 2>/dev/null)"
lc="$(printf '%s' "$prompt" | tr "[:upper:]" "[:lower:]")"

remind=""

# "Is X built yet" questions: the status board is the authoritative answer, read it, never recall.
if printf '%s' "$lc" | grep -qiE "do we have|is [a-z].* in yet|have we (built|got|added|done)|is [a-z].* (done|built|implemented|working|wired) yet|can (it|they|we|people|beings|a being|creatures|agents) [a-z].* yet|does the sim|already (have|built|support)|is that (built|done|in)"; then
  remind="${remind}[substrate-first] This reads as a \"is X built yet\" question. Answer by READING docs/working/CONSENSUS_ROADMAP.md (the authoritative status board, the single \"is X done\" lookup) and quoting the board line, NOT from memory. If the board is silent on X, say so and check the code; do not guess.
"
fi

# Value-authoring intent: force the derive-first challenge against the WHOLE substrate map.
if printf '%s' "$lc" | grep -qiE "set (this|the|a|that) value|what value|need a (number|value|constant)|hardcode|author (a|the|this|that) (value|number|constant)|reserved value|can'?t derive|cannot derive|pick a (number|value)|1 ?y ?= ?365|365 ?days?|just set|owner (decision|call) on (a|the) value|we have to set"; then
  remind="${remind}[derive-first] This reads as value-authoring. Before authoring OR flagging ANY value owner-set, READ docs/working/PHYSICS_FLOOR_REGISTRY.md (the whole physics-substrate map: the authored floor AND the deriving substrates, orbital mechanics / hydrology / metabolism / matter cycle / time-space) and prove IN WRITING that no substrate derives it. Authoring a derivable value is a defect under the value-authoring line; the honest output when a substrate is truly missing is \"build the substrate\", not \"set the value\".
"
fi

if [ -n "$remind" ]; then
  printf '%s' "$remind"
fi
exit 0
