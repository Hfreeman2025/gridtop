#!/usr/bin/env python3
"""Owned controlling-PTY checks. The lifecycle sources are synthetic; the tours are real dated dry runs."""
import argparse,codecs,errno,fcntl,json,sys,os,pty,re,resource,select,signal,struct,subprocess,termios,time
from pathlib import Path
ROOT=Path(__file__).resolve().parents[1]
OUT=ROOT/'shots/v3';OUT.mkdir(parents=True,exist_ok=True)

def read_pty(fd):
 try:return os.read(fd,65536)
 except OSError as e:
  if e.errno==errno.EIO:return b''
  raise

def controlling_terminal():
 os.setsid();fcntl.ioctl(0,termios.TIOCSCTTY,0)

def run(name,command,w,h,env,action,expected,synthetic=False):
 master,slave=pty.openpty();fcntl.ioctl(slave,termios.TIOCSWINSZ,struct.pack('HHHH',h,w,0,0))
 before=termios.tcgetattr(slave);start=time.monotonic();usage=resource.getrusage(resource.RUSAGE_CHILDREN)
 wrapper="import os,sys,json,signal,subprocess; signal.signal(signal.SIGINT,signal.SIG_IGN); p=subprocess.Popen(json.loads(sys.argv[1])); print('WRAPPER_CHILD:'+str(p.pid),flush=True); code=p.wait(); print('WRAPPER_FINISHED:'+str(code),flush=True); sys.stdin.buffer.read(1); sys.exit(code)"
 launched=[sys.executable,'-c',wrapper,json.dumps(command)] if synthetic else command
 p=subprocess.Popen(launched,stdin=slave,stdout=slave,stderr=slave,env={**os.environ,'TERM':'xterm-256color','TMPDIR':str(ROOT/'.tmp'),**env},preexec_fn=controlling_terminal if synthetic else None)
 output=bytearray();events=[];decoder=codecs.getincrementaldecoder('utf-8')();ready=None;sent=False;resized=False;after=None;released=False
 try:
  while p.poll() is None:
   if time.monotonic()-start>20:raise RuntimeError(f'{name}: timed out')
   if select.select([master],[],[],.01)[0]:
    chunk=read_pty(master)
    if not chunk:break
    output.extend(chunk);text=decoder.decode(chunk)
    if text:events.append([round(time.monotonic()-start,6),'o',text])
   if synthetic and b'WRAPPER_FINISHED:' in output and not released:
    after=termios.tcgetattr(slave);os.write(master,b'\n');released=True
   if ready is None and b'\x1b[?2026l' in output:ready=time.monotonic()
   if action=='demo' and ready and not sent and time.monotonic()-ready>6:os.kill(p.pid,signal.SIGINT);sent=True
   elif action=='stdin' and re.search(rb'CHILD_READY:\d+\r?\n',output) and not sent:os.write(master,b'hello from terminal\n');sent=True
   elif action=='keyboard' and re.search(rb'CHILD_READY:\d+\r?\n',output) and not sent:os.write(master,b'\x03');sent=True
   elif action=='term' and re.search(rb'CHILD_READY:\d+\r?\n',output) and not sent:os.kill(int(re.search(rb'WRAPPER_CHILD:(\d+)',output)[1]),signal.SIGTERM);sent=True
   elif action in ('cancel','resize') and b'versus starting now' in output and not sent:
    if action=='resize' and not resized:
     fcntl.ioctl(slave,termios.TIOCSWINSZ,struct.pack('HHHH',28,38,0,0));os.kill(int(re.search(rb'WRAPPER_CHILD:(\d+)',output)[1]),signal.SIGWINCH);resized=True
    elif action=='cancel' or re.search(rb'\x1b\[28;[0-9]+H',output):os.write(master,b'\x03');sent=True
  while select.select([master],[],[],0)[0]:
   chunk=read_pty(master)
   if not chunk:break
   output.extend(chunk);text=decoder.decode(chunk)
   if text:events.append([round(time.monotonic()-start,6),'o',text])
  p.wait(timeout=2)
  if after is None:after=termios.tcgetattr(slave)
  before[3]&=~getattr(termios,'PENDIN',0);after[3]&=~getattr(termios,'PENDIN',0)
  assert before==after,(name,'terminal flags changed')
  assert p.returncode==expected,(name,p.returncode,output[-2000:])
  assert b'\x1b[?1049l' in output and b'\x1b[?25h' in output,(name,'terminal not restored')
  if action=='stdin':assert b'ECHO:hello from terminal' in output
  if action=='demo':assert b'RECORDING' in output and not (Path(env['GRIDTOP_STATE_DIR'])/'runs').exists()
  directory=OUT/('lifecycle' if synthetic else name);directory.mkdir(parents=True,exist_ok=True)
  (directory/f'{name}.ansi').write_bytes(output)
  if not synthetic:
   header={'version':2,'width':w,'height':h,'title':'gridtop v3 · recorded 1 Nov 2026 forecast · dry run, no launch','env':{'TERM':'xterm-256color'}}
   (directory/'gridtop.cast').write_text('\n'.join(json.dumps(v,ensure_ascii=False) for v in [header,*events])+'\n')
  after_usage=resource.getrusage(resource.RUSAGE_CHILDREN);cpu=after_usage.ru_utime+after_usage.ru_stime-usage.ru_utime-usage.ru_stime;wall=time.monotonic()-start
  result={'name':name,'width':w,'height':h,'exit':p.returncode,'terminal_restored':True,'controlling_terminal':synthetic,'synthetic_source':synthetic,'first_complete_frame_ms':round((ready-start)*1000,2) if ready else None,'wall_seconds':round(wall,3),'cpu_percent_one_core':round(cpu/wall*100,2),'resize_checked':resized}
  if not synthetic:(directory/'metrics.json').write_text(json.dumps(result,indent=2)+'\n')
  return result
 finally:
  if p.poll() is None:
   child=re.search(rb'CHILD_READY:(\d+)',output)
   if child:
    try:os.killpg(int(child[1]),signal.SIGKILL)
    except ProcessLookupError:pass
   wrapper_child=re.search(rb'WRAPPER_CHILD:(\d+)',output)
   if wrapper_child:
    try:os.kill(int(wrapper_child[1]),signal.SIGKILL)
    except ProcessLookupError:pass
   p.kill();p.wait()
  os.close(master);os.close(slave)

def main():
 parser=argparse.ArgumentParser();parser.add_argument('--lifecycle-only',action='store_true');args=parser.parse_args()
 results=[];run_id=str(time.time_ns())
 if not args.lifecycle_only:
  for name,w,h,theme in [('wide',164,50,'phosphor'),('compact',80,24,'mono'),('phone',44,48,'amber')]:
   cmd=[str(ROOT/'target/release/gridtop'),'run','--demo','--dry-run','--watch','--theme',theme,'--for','2h','--watts','800','--','cargo','test','--release']
   results.append(run(name,cmd,w,h,{'GRIDTOP_STATE_DIR':str(ROOT/'.tmp/v3-dry-state'/name)},'demo',130))
 helpers=[p for p in (ROOT/'target/debug/deps').glob('carbon_flow-*') if p.is_file() and os.access(p,os.X_OK)]
 helper=max(helpers,key=lambda p:p.stat().st_mtime)
 for name,mode,action in [('stdin-handoff','stdin','stdin'),('keyboard-sigint','sigint','keyboard'),('forward-sigterm','sigterm','term'),('process-tree','tree','term'),('waiting-cancel','wait','cancel'),('resize','wait','resize'),('api-failure','unavailable','none'),('raw-modes','raw','none')]:
  directory=ROOT/'.tmp/v3-native'/run_id/name;directory.mkdir(parents=True,exist_ok=True)
  env={'GRIDTOP_FLOW_MODE':mode,'GRIDTOP_FLOW_ROOT':str(directory),'GRIDTOP_STATE_DIR':str(directory/'state')}
  cmd=[str(helper),'--ignored','--exact','orchestration_helper','--nocapture']
  result=run(name,cmd,164,50,env,action,0,True)
  records=list((directory/'state/runs').glob('*.json'));assert len(records)==1
  r=json.loads(records[0].read_text());result['ledger_status']=r['status'];result['child_exit']=r['exit_code']
  if mode=='tree':assert (directory/'executed').read_text()=='grandchild got SIGTERM'
  results.append(result)
 (OUT/'native-checks.json').write_text(json.dumps(results,indent=2)+'\n');print(json.dumps(results,indent=2))
if __name__=='__main__':main()
