#!/usr/bin/env python3
"""Capture an actual Rio window on an owned virtual X server, without devices.
Requires Xvfb, xdotool, Pillow, the locally extracted Rio and generated panel font.
No installation, network access, owner process startup or host configuration writes.
"""
import argparse,os,subprocess,time,tempfile
from pathlib import Path
from PIL import ImageGrab
ROOT=Path(__file__).resolve().parents[1]
p=argparse.ArgumentParser()
p.add_argument('--name',default='specimen')
p.add_argument('--font-size',type=int,default=20)
p.add_argument('--family',default='SHR Panel Mono')
p.add_argument('--system-fonts-only',action='store_true')
p.add_argument('--cpu-terminal',action='store_true')
p.add_argument('--width',type=int,default=1560)
p.add_argument('--height',type=int,default=900)
p.add_argument('--keys',default='a')
p.add_argument('--click')
p.add_argument('--resize')
p.add_argument('--settle-seconds',type=float,default=.4)
p.add_argument('--command',nargs=argparse.REMAINDER)
a=p.parse_args()
assert a.name.replace('-','').replace('_','').isalnum()
base=ROOT/'artifacts/instrument-panel';base.mkdir(parents=True,exist_ok=True)
scratch=tempfile.TemporaryDirectory(prefix='shr-panel-preview-');run=Path(scratch.name)
font=base/'fonts/SHRPanelMono.ttf';rio=base/'tools/rio/usr/bin/rio'
if not font.is_file() or not rio.is_file():p.error('build the documented local font and extract Rio first')
(run/'config.toml').write_text(f'''[window]
width = {a.width}
height = {a.height}
decorations = "Disabled"
[fonts]
family = "{a.family}"
size = {a.font_size}
[renderer]
backend = "Vulkan"
use-cpu = {str(a.cpu_terminal).lower()}
''')
font_directory='' if a.system_fonts_only else f'<dir>{font.parent}</dir>'
(run/'fonts.conf').write_text(f'<fontconfig><include>/etc/fonts/fonts.conf</include>{font_directory}<cachedir>{run}/font-cache</cachedir></fontconfig>')
number=next(n for n in range(94,110) if not Path(f'/tmp/.X11-unix/X{n}').exists() and not Path(f'/tmp/.X{n}-lock').exists())
display=f':{number}'
env=os.environ.copy();env.pop('NO_COLOR',None);env.pop('WAYLAND_DISPLAY',None)
env.update(DISPLAY=display,RIO_CONFIG_HOME=str(run),FONTCONFIG_FILE=str(run/'fonts.conf'),SHR_PANEL_FONT='SHR Panel Mono:0.3',VK_DRIVER_FILES='/usr/share/vulkan/icd.d/lvp_icd.json',VK_ICD_FILENAMES='/usr/share/vulkan/icd.d/lvp_icd.json',WINIT_UNIX_BACKEND='x11')
if a.system_fonts_only:env.pop('SHR_PANEL_FONT',None)
x=None;r=None
try:
 with (run/'xvfb.log').open('w') as log:x=subprocess.Popen(['Xvfb',display,'-screen','0',f'{a.width}x{a.height}x24','-nolisten','tcp'],stdout=log,stderr=log)
 for _ in range(100):
  if Path(f'/tmp/.X11-unix/X{number}').exists():break
  if x.poll() is not None:raise RuntimeError('virtual server failed')
  time.sleep(.02)
 command=a.command or [str(ROOT/'target/debug/panel-specimen'),'--font']
 with (run/'rio.log').open('w') as log:r=subprocess.Popen([str(rio),'-e',*command],env=env,stdout=log,stderr=log)
 window=None
 for _ in range(150):
  q=subprocess.run(['xdotool','search','--class','rio'],env=env,capture_output=True,text=True)
  if q.returncode==0:window=q.stdout.splitlines()[0];break
  if r.poll() is not None:raise RuntimeError('Rio exited; inspect rio.log')
  time.sleep(.02)
 if window is None:raise RuntimeError('window deadline')
 subprocess.run(['xdotool','windowfocus',window],env=env,check=True)
 # Window creation precedes the first software Vulkan presentation.
 for _ in range(200):
  shot=ImageGrab.grab(xdisplay=display)
  # Ignore the title bar: its antialiased text can appear before app output.
  colours=shot.crop((0,42,a.width,a.height)).getcolors(10000)
  if colours is None or len(colours)>16:break
  if r.poll() is not None:raise RuntimeError('Rio exited before presentation')
  time.sleep(.05)
 else:raise RuntimeError('first terminal presentation deadline')
 time.sleep(.6) # allow read-only provider/role attachment after the first paint
 if a.resize:
  rw,rh=a.resize.split(',');subprocess.run(['xdotool','windowsize',window,rw,rh],env=env,check=True);time.sleep(.2)
 if a.keys:
  for key in a.keys.split(','):
   subprocess.run(['xdotool','key',key],env=env,check=True)
   time.sleep(.12)
 if a.click:
  cx,cy=a.click.split(',');subprocess.run(['xdotool','mousemove',cx,cy,'click','1'],env=env,check=True)
 time.sleep(max(0,min(a.settle_seconds,10)))
 output=base/(a.name+'.png');ImageGrab.grab(xdisplay=display).save(output)
 print(output)
 subprocess.run(['xdotool','key','q' if any('panel-specimen' in token for token in command) else 'ctrl+c'],env=env,check=True)
 try:r.wait(timeout=3)
 except subprocess.TimeoutExpired:r.terminate();r.wait(timeout=3)
finally:
 if r is not None and r.poll() is None:r.terminate();r.wait(timeout=3)
 if x is not None and x.poll() is None:x.terminate();x.wait(timeout=3)
 scratch.cleanup()
