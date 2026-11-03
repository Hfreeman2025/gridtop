#!/usr/bin/env python3
"""Real, offline, scripted interaction recording of the public demo; no emulator fakery."""
import argparse,codecs,fcntl,json,os,pty,select,struct,subprocess,termios,time
from pathlib import Path
root=Path(__file__).resolve().parents[1]
parser=argparse.ArgumentParser()
parser.add_argument('--output',default='shots/v2')
parser.add_argument('--width',type=int,default=164)
parser.add_argument('--height',type=int,default=50)
args=parser.parse_args()
out=(root/args.output).resolve()
assert out.is_relative_to(root/'shots'), 'recordings must stay in project shots/'
out.mkdir(parents=True,exist_ok=True)
master,slave=pty.openpty();fcntl.ioctl(slave,termios.TIOCSWINSZ,struct.pack('HHHH',args.height,args.width,0,0))
before=termios.tcgetattr(slave);start=time.monotonic()
p=subprocess.Popen([str(root/'target/release/gridtop'),'--demo'],stdin=slave,stdout=slave,stderr=slave,env={**os.environ,'TERM':'xterm-256color'})
output=bytearray();decoder=codecs.getincrementaldecoder('utf-8')();events=[];ready=None
steps=[(.8,b'\x1b[C'*16),(1.3,f'\x1b[<0;{args.width-2};{args.height-5}M\x1b[<0;{args.width-2};{args.height-5}m'.encode()),(1.8,b'kkkk'),(2.4,b't'),(3.2,b't'),(4.,b't'),(4.8,b'?'),(5.6,b'\x1b'),(5.9,b'v'),(6.6,b' '),(7.4,b'mp'),(9.5,b'q')]
sent=[]
try:
 while p.poll() is None:
  if time.monotonic()-start>30:raise RuntimeError('tour timed out')
  if select.select([master],[],[],.01)[0]:
   chunk=os.read(master,65536);output.extend(chunk);decoded=decoder.decode(chunk)
   if decoded:events.append([round(time.monotonic()-start,6),'o',decoded])
  if ready is None and b'\x1b[?2026l' in output:ready=time.monotonic()
  if ready is not None and steps and time.monotonic()-ready>=steps[0][0]:
   when,data=steps.pop(0);os.write(master,data);sent.append({'seconds':when,'keys':repr(data)})
 p.wait();assert p.returncode==0
 while select.select([master],[],[],0)[0]:
  chunk=os.read(master,65536);output.extend(chunk);decoded=decoder.decode(chunk)
  if decoded:events.append([round(time.monotonic()-start,6),'o',decoded])
 after=termios.tcgetattr(slave);before[3]&=~getattr(termios,'PENDIN',0);after[3]&=~getattr(termios,'PENDIN',0)
 assert before==after and b'\x1b[?1049l' in output and b'\x1b[?25h' in output
 assert b'\x1b[7m' in output, 'cleanest-window reverse markers were not emitted'
 assert not steps, 'tour ended before all interactions'
 header={'version':2,'width':args.width,'height':args.height,'title':'gridtop · 14 real regions · recorded 1 Nov 2026','env':{'TERM':'xterm-256color'}}
 (out/'gridtop.cast').write_text('\n'.join(json.dumps(x,ensure_ascii=False) for x in [header,*events])+'\n')
 (out/'tour.ansi').write_bytes(output)
 metrics={'first_complete_terminal_frame_ms':round((ready-start)*1000,2),'duration_seconds':round(time.monotonic()-start,3),'steps':sent,'exit':p.returncode,'terminal_restored':True,'cursor_restored':True,'cleanest_reverse_attribute':True,'width':args.width,'height':args.height}
 (out/'tour-metrics.json').write_text(json.dumps(metrics,indent=2)+'\n');print(json.dumps(metrics))
finally:
 if p.poll() is None:p.terminate();p.wait(timeout=10)
 os.close(master);os.close(slave)
