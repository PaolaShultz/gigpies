#!/usr/bin/env python3
"""Opt-in 100-frame Rio/Xvfb presentation observation; no provider or devices.
Requires the documented local font and Rio executable. Frame markers
are observed in the actual X framebuffer, then ACKed through terminal input.
Readback/polling overhead is deliberately included, not called display latency.
"""
import argparse,json,os,statistics,subprocess,time,tempfile
from pathlib import Path
from PIL import ImageGrab
ROOT=Path(__file__).resolve().parents[1]
p=argparse.ArgumentParser();p.add_argument('scenes',type=Path);p.add_argument('--name',default='presentation');p.add_argument('--cpu-terminal',action='store_true');a=p.parse_args()
assert a.name.replace('-','').replace('_','').isalnum()
base=ROOT/'artifacts/instrument-panel';base.mkdir(parents=True,exist_ok=True)
scratch=tempfile.TemporaryDirectory(prefix='shr-panel-measure-');run=Path(scratch.name)
(run/'config.toml').write_text('[window]\nwidth = 1928\nheight = 1122\ndecorations = "Disabled"\n[fonts]\nfamily = "SHR Panel Mono"\nsize = 24\n[renderer]\nbackend = "Vulkan"\nuse-cpu = ' + str(a.cpu_terminal).lower() + '\n')
fontconf=run/'fonts.conf'
fontconf.write_text(f'<fontconfig><include>/etc/fonts/fonts.conf</include><dir>{base}/fonts</dir><cachedir>{run}/font-cache</cachedir></fontconfig>')
number=next(n for n in range(94,110) if not Path(f'/tmp/.X11-unix/X{n}').exists() and not Path(f'/tmp/.X{n}-lock').exists());display=f':{number}'
env=os.environ.copy();env.pop('NO_COLOR',None);env.pop('WAYLAND_DISPLAY',None)
env.update(DISPLAY=display,RIO_CONFIG_HOME=str(run),FONTCONFIG_FILE=str(fontconf),SHR_PANEL_FONT='SHR Panel Mono:0.3',VK_DRIVER_FILES='/usr/share/vulkan/icd.d/lvp_icd.json',VK_ICD_FILENAMES='/usr/share/vulkan/icd.d/lvp_icd.json',WINIT_UNIX_BACKEND='x11')
timing=run/'frames.txt';timing.unlink(missing_ok=True)
x=None;r=None;observed=[];started=time.monotonic_ns()
try:
 with (run/'xvfb.log').open('w') as log:x=subprocess.Popen(['Xvfb',display,'-screen','0','1928x1122x24','-nolisten','tcp'],stdout=log,stderr=log)
 for _ in range(100):
  if Path(f'/tmp/.X11-unix/X{number}').exists():break
  time.sleep(.02)
 with (run/'rio.log').open('w') as log:r=subprocess.Popen(['/usr/bin/time','-v','-o',str(run/'resources.txt'),str(base/'tools/rio/usr/bin/rio'),'-e',str(ROOT/'target/release/examples/present'),str(a.scenes.resolve()),str(timing)],env=env,stdout=log,stderr=log,start_new_session=True)
 for _ in range(250):
  q=subprocess.run(['xdotool','search','--class','rio'],env=env,capture_output=True,text=True)
  if q.returncode==0:
   subprocess.run(['xdotool','windowfocus',q.stdout.splitlines()[0]],env=env,check=True);break
  time.sleep(.02)
 for i in range(100):
  deadline=time.monotonic()+5;start=None
  while time.monotonic()<deadline:
   if timing.exists():
    lines=timing.read_text().splitlines()
    if len(lines)>i:start=int(lines[i].split()[1])
   if start is not None:
    frame=ImageGrab.grab(xdisplay=display)
    # Binary marker survives colour management/rounding; frame number is unique.
    def marker(y):
     rgb=frame.getpixel((124,y))
     if not (rgb[0]>180 and rgb[1]<80 and rgb[2]>180):return False
     for bit in range(7):
      rgb=frame.getpixel((364+240*bit,y));on=bool(i & (1<<bit))
      if (on and min(rgb)<180) or (not on and max(rgb)>100):return False
     return True
    if any(marker(y) for y in range(1042,1122,2)):
     now=time.monotonic_ns();observed.append(now-start)
     if i==0:startup=now-started
     subprocess.run(['xdotool','key','space'],env=env,check=True,stdout=subprocess.DEVNULL);break
   time.sleep(.002)
  else:
   raise RuntimeError(f'frame {i} was not observed')
 r.wait(timeout=5)
 result={'frames':len(observed),'median_ms':statistics.median(observed)/1e6,'p95_ms':sorted(observed)[94]/1e6,'startup_to_first_visible_ms':startup/1e6,'renderer':'Rio tiny-skia' if a.cpu_terminal else 'Rio Vulkan lavapipe','limits':'X framebuffer; includes framebuffer readback/polling and scheduling, excludes physical scanout. Marker adds one changed row even for unchanged scenes.'}
 result['terminal_cells']=Path(str(timing)+'.size').read_text().strip()
 result['resources']={line.strip().split(': ',1)[0]:line.strip().split(': ',1)[1] for line in (run/'resources.txt').read_text().splitlines() if ': ' in line and any(k in line for k in ['User time','System time','Maximum resident'])}
 (base/(a.name+'.json')).write_text(json.dumps(result,indent=2)+'\n');print(json.dumps(result))
finally:
 if r is not None and r.poll() is None:
  import signal
  os.killpg(r.pid,signal.SIGTERM);r.wait(timeout=5)
 if x is not None and x.poll() is None:x.terminate();x.wait(timeout=5)
 scratch.cleanup()
