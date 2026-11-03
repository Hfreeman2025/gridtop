#!/usr/bin/env python3
"""One read-only public API check of the native recording-to-live transition."""
import fcntl,json,os,pty,select,signal,struct,subprocess,termios,time
from pathlib import Path
root=Path(__file__).resolve().parents[1];m,s=pty.openpty()
fcntl.ioctl(s,termios.TIOCSWINSZ,struct.pack('HHHH',50,164,0,0));before=termios.tcgetattr(s)
start=time.monotonic();p=subprocess.Popen([str(root/'target/release/gridtop')],stdin=s,stdout=s,stderr=s,env={**os.environ,'TERM':'xterm-256color'})
output=bytearray();first=None;live=None;preview=False
try:
 while time.monotonic()-start<25:
  if select.select([m],[],[],.05)[0]:output.extend(os.read(m,65536))
  if first is None and b'\x1b[?2026l' in output:
   first=time.monotonic();frame=output.split(b'\x1b[?2026l',1)[0];preview=b'RECORDING' in frame and b'connecting' in frame
  # Changed cells may omit unchanged letters in 'LIVE DATA'; 'checked' is unique to the fresh-API branch.
  if b'checked' in output and output.endswith(b'\x1b[?2026l'):live=time.monotonic();break
  if p.poll() is not None:break
 os.write(m,b'q');p.wait(timeout=10)
 after=termios.tcgetattr(s);before[3]&=~getattr(termios,'PENDIN',0);after[3]&=~getattr(termios,'PENDIN',0);assert before==after
 result={'first_complete_frame_ms':round((first-start)*1000,2) if first else None,'dated_startup_preview_observed':preview,'first_live_data_frame_ms':round((live-start)*1000,2) if live else None,'live_data_received':live is not None,'terminal_restored':True,'exit':p.returncode}
 (root/'shots/v2/live-preview.json').write_text(json.dumps(result,indent=2)+'\n');(root/'shots/v2/live-preview.ansi').write_bytes(output);print(json.dumps(result));assert preview and live is not None
finally:
 if p.poll() is None:p.send_signal(signal.SIGINT);p.wait(timeout=10)
 os.close(m);os.close(s)
