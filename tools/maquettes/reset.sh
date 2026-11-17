#!/bin/bash
# Remet le site d'essai à neuf et relance le pont (il garde le document en mémoire).
SP=/home/claude/sp
ps aux | awk '/revel canvas-serve/ && !/awk/{print $2}' | xargs -r kill; sleep 0.5
rm -rf $SP/site && mkdir -p $SP/site && tar -C /home/claude/cobaye-0dd004c -cf - . | tar -C $SP/site -xf -
cd /home/claude/revel
(REVEL_MODELS=/nonexistent nohup target/release/revel canvas-serve $SP/site --port 8174 --bridge > $SP/pont.log 2>&1 &)
sleep 3; tail -1 $SP/pont.log
