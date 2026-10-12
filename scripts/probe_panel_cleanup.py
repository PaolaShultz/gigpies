#!/usr/bin/env python3
"""Read-only terminal query of the registered experimental repertoire after normal cleanup.
Run inside the same Rio PTY after panel-specimen --glyphs exits, with system fonts
only. Uses bounded replies and restores termios; does not clear or upload glyphs.
"""
import os,termios,tty,select,time,json
from pathlib import Path
import argparse
p=argparse.ArgumentParser(description=__doc__);p.add_argument('output',type=Path);args=p.parse_args()
registry=Path(__file__).resolve().parents[1]/'src/instrument-panel/assets/registry.json'
codepoints=[int(t['codepoint'][2:],16) for t in json.loads(registry.read_text())['tiles']]
fd=0;old=termios.tcgetattr(fd);replies=[]
try:
 tty.setraw(fd)
 for cp in codepoints:
  os.write(1,f'\x1b_25a1;q;cp={cp:x}\x1b\\'.encode());buf=b'';deadline=time.monotonic()+.5
  while time.monotonic()<deadline:
   ready,_,_=select.select([fd],[],[],.05)
   if ready:buf+=os.read(fd,4096)
   if b'\x1b\\' in buf:break
  replies.append(buf.decode(errors='replace'))
finally:termios.tcsetattr(fd,termios.TCSANOW,old)
args.output.write_text(json.dumps({'queried':len(codepoints),'empty':sum('status=\x1b' in r for r in replies)},indent=2)+'\n')

if len(replies)!=len(codepoints) or any('status=\x1b' not in r for r in replies):raise SystemExit('not all experimental slots are empty')
