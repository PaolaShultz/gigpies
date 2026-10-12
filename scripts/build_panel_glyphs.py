#!/usr/bin/env python3
"""Original MIT panel geometry -> deterministic COLRv0 Glyph Protocol containers.
Geometry generation has no downloads or dependencies. Optional font assembly
uses fontTools and a supplied licensed foundation; it never changes host fonts.
Coordinates are 1000 x 1000, y-up, mapped to one installed-font cell.
"""
import base64
import json
from pathlib import Path
import struct

ROOT = Path(__file__).resolve().parents[1]
OUT = ROOT / 'src/instrument-panel'

def u16(n):
    return struct.pack('>H', n)

def rect(x, y, w, h):
    return [(x,y), (x,y+h), (x+w,y+h), (x+w,y)]

def octagon(x,y,w,h,r):
    return [(x+r,y),(x,y+r),(x,y+h-r),(x+r,y+h),(x+w-r,y+h),(x+w,y+h-r),(x+w,y+r),(x+w-r,y)]

def outline(points):
    xs, ys = zip(*points)
    data = struct.pack('>hhhhh',1,min(xs),min(ys),max(xs),max(ys))
    data += u16(len(points)-1) + u16(0) + bytes([1]*len(points))
    for coords in [xs,ys]:
        prev=0
        for n in coords:
            data += struct.pack('>h',n-prev)
            prev=n
    return data

def container(layers):
    # Empty base glyph 0 plus simple outlines 1..n. Full cell bounds are
    # established by the first opaque background layer, including LED off states.
    glyphs=[struct.pack('>hhhhh',0,0,0,1000,1000)] + [outline(p) for p,c in layers]
    n=len(layers)
    assert n <= 16
    colr=struct.pack('>HHIIH',0,1,14,20,n) + struct.pack('>HHH',0,0,n)
    colr+=b''.join(struct.pack('>HH',i+1,i) for i in range(n))
    cpal=struct.pack('>HHHHIH',0,n,1,n,14,0)
    cpal+=b''.join(bytes((c[2],c[1],c[0],255)) for p,c in layers)
    data=u16(len(glyphs))+b''.join(u16(len(g))+g for g in glyphs)
    data+=u16(len(colr))+colr+u16(len(cpal))+cpal
    assert len(data)<65536
    return base64.b64encode(data).decode()

PLATE=(49,55,63)
DARK=(9,13,18)
TILES=[]
def tile(name,layers):
    TILES.append((name,[(rect(0,0,1000,1000),PLATE)]+layers))

# Edge strips keep metallic shading on boundaries, leaving centre label cells
# ordinary text against a flat matching surface, never text baked into glyphs.
for state,hi,lo,face in [('raised',(200,210,218),(12,17,24),(89,98,109)),('pressed',(13,18,24),(151,161,172),(39,46,55)),('focused',(126,198,244),(24,68,111),(89,98,109)),('disabled',(80,85,91),(31,35,41),(57,62,69)),('latched',(152,190,161),(19,55,30),(54,76,63)),('warning',(239,174,62),(94,47,13),(98,70,39)),('unavailable',(67,72,79),(25,29,35),(39,44,51))]:
    for edge in ['top','bottom','left','right']:
        layers=[(rect(0,0,1000,1000),face)]
        if edge=='top':
            layers += [(rect(0,700,1000,300),lo),(rect(0,800,1000,120),hi),(rect(0,530,1000,170),tuple((a+b)//2 for a,b in zip(hi,face)))]
        elif edge=='bottom':
            layers += [(rect(0,0,1000,350),lo),(rect(0,350,1000,100),tuple((a+b)//2 for a,b in zip(lo,face)))]
        elif edge=='left': layers += [(rect(0,0,170,1000),hi),(rect(170,0,100,1000),tuple((a+b)//2 for a,b in zip(hi,face)))]
        else: layers += [(rect(730,0,270,1000),lo)]
        tile(state+'_'+edge,layers)
        if edge in ['top','bottom']:
            for side in ['left','right']:
                corner=list(layers)
                if edge=='top':
                    pts=[(0,1000),(0,550),(450,1000)] if side=='left' else [(550,1000),(1000,550),(1000,1000)]
                else:
                    pts=[(0,0),(450,0),(0,450)] if side=='left' else [(550,0),(1000,0),(1000,450)]
                corner.append((pts,PLATE))
                tile(state+'_'+edge+'_'+side,corner)
for name,colour in [('green',(65,236,129)),('yellow',(255,203,66)),('red',(255,64,60)),('blue',(74,168,255))]:
    for on in [False,True]:
        layers=[(octagon(45,220,910,560,120),(13,18,24)),(octagon(85,270,830,480,110),(124,134,143)),(octagon(125,300,750,410,100),(8,13,17))]
        # Fully opaque bounded bands emulate glow; no neighbour overdraw.
        for inset,factor in [(0,.13),(60,.28),(110,.65),(155,1.0)]:
            c=tuple(round(v*(factor if on else factor*.16)) for v in colour)
            layers.append((octagon(150+inset,315+inset//2,700-2*inset,380-inset,50),c))
        layers.append((rect(330,550,230,38),(221,240,247) if on else (54,66,74)))
        tile(name+('_on' if on else '_off'),layers)
for i in range(9):
    layers=[(rect(80,100,840,800),DARK)]
    if i:
        for j in range(i):
            layers.append((rect(130,130+j*85,740,75),(85+18*j,21+3*j,30+2*j)))
    tile('red_level_'+str(i),layers)
for name,layers in [('well_top',[(rect(0,400,1000,600),DARK),(rect(0,860,1000,140),(17,23,30))]),('well_bottom',[(rect(0,0,1000,700),DARK),(rect(0,60,1000,100),(120,129,138))]),('fader_track',[(rect(400,0,200,1000),DARK),(rect(600,0,80,1000),(109,119,130))]),('fader_cap',[(octagon(30,150,940,700,100),(14,19,25)),(octagon(70,230,850,600,65),(173,182,191)),(rect(110,650,770,100),(233,238,243)),(rect(130,400,740,50),(47,55,64))])]: tile(name,layers)
# Channel equipment: reusable multi-cell encoders and slider caps. Coordinates
# are physical (one cell is 1 x 2); clipping makes every join exactly coincident.
import math
RACK=(43,47,52)
def ellipse(cx,cy,rx,ry,n=48):
    return [(round(cx+rx*math.cos(2*math.pi*i/n)),round(cy+ry*math.sin(2*math.pi*i/n))) for i in range(n)]
def clip_polygon(points,x0,y0,x1,y1):
    for axis,bound,sign in [(0,x0,1),(0,x1,-1),(1,y0,1),(1,y1,-1)]:
        result=[]
        if not points:break
        prev=points[-1]
        for point in points:
            pi=(prev[axis]-bound)*sign>=0;ci=(point[axis]-bound)*sign>=0
            if pi!=ci:
                t=(bound-prev[axis])/(point[axis]-prev[axis])
                result.append((prev[0]+t*(point[0]-prev[0]),prev[1]+t*(point[1]-prev[1])))
            if ci:result.append(point)
            prev=point
        points=result
    return points

def mosaic(name,w,h,layers):
    for row in range(h):
        for col in range(w):
            out=[(rect(0,0,1000,1000),RACK)]
            x0=col*1000;y0=(h-1-row)*2000
            for points,c in layers:
                clipped=clip_polygon(points,x0,y0,x0+1000,y0+2000)
                if len(clipped)>2:
                    out.append(([(round(x-x0),round((y-y0)/2)) for x,y in clipped],c))
            TILES.append((f'{name}_{row}_{col}',out))

for step in range(17):
    layers=[(ellipse(3000,2770,2770,2770),(12,15,19)),
            (ellipse(2920,3140,2660,2660),(157,166,174)),
            (ellipse(3030,2930,2470,2470),(68,75,84)),
            (ellipse(3000,3000,2300,2300),(202,210,218)),
            (ellipse(3000,3070,2070,2070),(94,104,114)),
            (ellipse(3000,3150,1860,1860),(58,65,73)),
            (ellipse(3000,3310,1650,1570),(67,74,82))]
    theta=math.radians(225-step*270/16)
    dx,dy=math.cos(theta),math.sin(theta)
    px,py=-dy*180,dx*180
    layers.append(([(round(3000+dx*r+px*k),round(3000+dy*r+py*k)) for r,k in [(450,1),(1850,1),(1850,-1),(450,-1)]],(218,235,231)))
    mosaic(f'encoder_{step}',6,3,layers)

mosaic('slider',6,2,[(octagon(0,0,6000,3800,450),(9,12,16)),
 (octagon(120,350,5720,3500,300),(105,114,125)),
 (octagon(160,730,5570,3100,220),(177,185,192)),
 (rect(250,3330,5400,360),(227,232,234)),
 (rect(260,2670,5380,400),(190,198,203)),
 (rect(260,1250,5380,1000),(140,151,162)),
 (rect(260,1790,5380,170),(36,44,53)),
 (rect(450,1920,5000,80),(231,238,242))])

# Complete boundary profiles, including true corner intersections; labels stay text.
for state,face,hi,lo in [('key',(87,94,103),(181,193,202),(13,17,22)),
 ('active',(57,78,89),(128,207,224),(13,30,40)),
 ('redkey',(113,37,43),(236,108,110),(40,12,17))]:
    for edge in ['top','bottom','left','right','tl','tr','bl','br']:
        lay=[(rect(0,0,1000,1000),face)]
        if edge in ['top','tl','tr']:
            lay += [(rect(0,850,1000,150),RACK),(rect(0,740,1000,110),hi),(rect(0,570,1000,170),tuple((a+b)//2 for a,b in zip(hi,face)))]
        if edge in ['bottom','bl','br']:
            lay += [(rect(0,0,1000,250),RACK),(rect(0,250,1000,210),lo),(rect(0,460,1000,90),tuple((a+b)//2 for a,b in zip(lo,face)))]
        if edge in ['left','tl','bl']:
            lay += [(rect(0,0,180,1000),RACK),(rect(180,250 if edge=='bl' else 0,90,490 if edge=='tl' else 750 if edge=='bl' else 1000),hi)]
        if edge in ['right','tr','br']:
            lay += [(rect(870,0,130,1000),RACK),(rect(730,250 if edge=='br' else 0,140,490 if edge=='tr' else 750 if edge=='br' else 1000),lo)]
        # Small rounded outer corner; never the disconnected triangular cuts of v0.1.
        TILES.append((f'{state}_{edge}',lay))

lines=['// Generated by scripts/build_panel_glyphs.py; original MIT artwork.','pub const BASE: u32 = 0x101000;','#[rustfmt::skip]','pub const ASSETS: &[(&str, &str)] = &[']
# Three-cell indicator lenses: visible bezels when dark, bounded glow when lit.
# Appended to preserve every previously assigned codepoint.
for name, colour in [('green',(65,236,129)),('yellow',(255,203,66)),('red',(255,64,60)),('blue',(74,168,255))]:
    for on in [False,True]:
        layers=[(octagon(70,220,2860,1520,450),(10,14,18)),
                (octagon(150,370,2700,1410,400),(144,154,163)),
                (octagon(230,370,2540,1260,350),(20,25,30))]
        for inset, factor in [(0,.18),(100,.32),(230,.65),(370,1.)]:
            c=tuple(round(v*(factor if on else factor*.12)) for v in colour)
            layers.append((octagon(310+inset,440+inset//2,2380-2*inset,1110-inset,250),c))
        layers.append((rect(920,1350,900,100),(228,246,240) if on else (64,76,81)))
        mosaic('lens_'+name+('_on' if on else '_off'),3,1,layers)

registry=[]
for i,(name,layers) in enumerate(TILES):
    lines.append('    ("'+name+'", "'+container(layers)+'"),')
    registry.append({'name':name,'codepoint':f'U+{0x101000+i:06X}','layers':len(layers)})
lines.append('];')
(OUT/'src/assets.rs').write_text('\n'.join(lines)+'\n')
(OUT/'assets/registry.json').write_text(json.dumps({'format':'colrv0','upm':1000,'advance':1000,'line_height':1000,'width':1,'size':'stretch','tiles':registry},indent=2)+'\n')
print(f'{len(TILES)} tiles; {sum(len(container(l)) for _,l in TILES)} base64 bytes')

# Optional installed-font experiment. Output stays in the requested artifact dir.
# DejaVu's original licence/name records are retained; derivative family renamed.
if __name__ == '__main__':
    import argparse
    parser=argparse.ArgumentParser()
    parser.add_argument('--font',type=Path)
    parser.add_argument('--colrv1',action='store_true')
    parser.add_argument('--out',type=Path)
    args=parser.parse_args()
    if args.font:
        from fontTools.ttLib import TTFont
        from fontTools.pens.ttGlyphPen import TTGlyphPen
        from fontTools.colorLib.builder import buildCOLR, buildCPAL
        font=TTFont(args.font,recalcTimestamp=False)
        order=list(font.getGlyphOrder())
        # Exact half-em advance and one-em line box avoid fractional cell seams
        # at even pixel font sizes. Transform the licensed foundation consistently.
        from fontTools.pens.recordingPen import DecomposingRecordingPen
        from fontTools.pens.transformPen import TransformPen
        original_aw=font['hmtx']['M'][0]
        original_ascent,original_descent=font['hhea'].ascent,font['hhea'].descent
        aw=font['head'].unitsPerEm//2
        ascent,descent=1600,-448
        sx=aw/original_aw;sy=(ascent-descent)/(original_ascent-original_descent)
        glyphset=font.getGlyphSet();converted={}
        for glyph_name in order:
            recording=DecomposingRecordingPen(glyphset);glyphset[glyph_name].draw(recording)
            pen=TTGlyphPen(None)
            recording.replay(TransformPen(pen,(sx,0,0,sy,0,descent-original_descent*sy)))
            converted[glyph_name]=pen.glyph()
        for glyph_name,glyph in converted.items():
            font['glyf'][glyph_name]=glyph
            old_width,old_lsb=font['hmtx'][glyph_name]
            font['hmtx'][glyph_name]=(round(old_width*sx),round(old_lsb*sx))
        font['hhea'].ascent=ascent;font['hhea'].descent=descent;font['hhea'].lineGap=0
        font['OS/2'].sTypoAscender=ascent;font['OS/2'].sTypoDescender=descent;font['OS/2'].sTypoLineGap=0
        font['OS/2'].usWinAscent=ascent;font['OS/2'].usWinDescent=-descent
        palette=[]; colors={}; mapping={}
        for i,(name,layers) in enumerate(TILES):
            base='panel_'+name
            mapping[0x101000+i]=base
            entries=[]
            for j,(points,c) in enumerate(layers):
                layer=base+'_'+str(j);pen=TTGlyphPen(None)
                pts=[(round(x*aw/1000),round(descent+y*(ascent-descent)/1000)) for x,y in points]
                pen.moveTo(pts[0])
                for point in pts[1:]:pen.lineTo(point)
                pen.closePath();font['glyf'][layer]=pen.glyph();font['hmtx'][layer]=(aw,min(x for x,y in pts));order.append(layer)
                rgba=tuple(v/255 for v in c)+(1.0,)
                if rgba not in palette:palette.append(rgba)
                entries.append((layer,palette.index(rgba)))
            font['glyf'][base]=font['glyf'][entries[0][0]];font['hmtx'][base]=(aw,0);order.append(base);colors[base]=entries
        font.setGlyphOrder(order)
        for t in font['cmap'].tables:
            if t.format==12:t.cmap.update(mapping)
        if args.colrv1:
            colors={base:{'Format':1,'Layers':[{'Format':10,'Glyph':glyph,'Paint':{'Format':2,'PaletteIndex':index,'Alpha':1.0}} for glyph,index in layers]} for base,layers in colors.items()}
        font['COLR']=buildCOLR(colors,version=1 if args.colrv1 else 0,glyphMap=font.getReverseGlyphMap())
        font['CPAL']=buildCPAL([palette])
        for nameid,text in [(1,'SHR Panel Mono'),(2,'Regular'),(3,'SHR Panel Mono 0.3'),(4,'SHR Panel Mono'),(6,'SHRPanelMono'),(16,'SHR Panel Mono'),(17,'Regular')]:
            if args.colrv1:text=text.replace('SHR Panel Mono','SHR Panel Mono V1').replace('SHRPanelMono','SHRPanelMonoV1')
            font['name'].setName(text,nameid,3,1,0x409)
            font['name'].setName(text,nameid,1,0,0)
        if not args.out:parser.error('--out required with --font')
        args.out.parent.mkdir(parents=True,exist_ok=True)
        font.save(args.out)
        print(f'Font {args.out}: aw={aw}, ascent={ascent}, descent={descent}, {len(palette)} colours')
