#define _XOPEN_SOURCE 700
// MIT. Bounded Notcurses 3.0.11 transport comparison, independent terminal owner.
// Takes one 936x864 RGBA plate from the Channel authoring source and a stats path.
#include <notcurses/notcurses.h>
#include <stdio.h>
#include <stdlib.h>
#include <time.h>
#include <unistd.h>
static double now(void){struct timespec t;clock_gettime(CLOCK_MONOTONIC,&t);return t.tv_sec+t.tv_nsec/1e9;}
int main(int argc,char**argv){
 if(argc!=3)return 2;
 FILE*f=fopen(argv[1],"rb");if(!f)return 3;
 unsigned char*rgba=malloc(936*864*4);if(fread(rgba,4,936*864,f)!=936*864)return 4;fclose(f);
 notcurses_options opts={.flags=NCOPTION_SUPPRESS_BANNERS|NCOPTION_NO_FONT_CHANGES};
 struct notcurses*nc=notcurses_core_init(&opts,stdout);if(!nc)return 5;
 struct ncplane*std=notcurses_stdplane(nc);
 ncplane_options po={.y=2,.x=2,.rows=36,.cols=78};
 struct ncplane*plate=ncplane_create(std,&po);
 uint64_t ul=0,ll=0;ncchannels_set_bg_rgb(&ul,0xbac2ca);ncchannels_set_bg_rgb(&ll,0x626c76);
 ncplane_gradient(plate,0,0,18,78," ",0,ul,ul,ll,ll);
 ncplane_gradient2x1(plate,18,0,18,78,NCCHANNEL_INITIALIZER(186,194,202),NCCHANNEL_INITIALIZER(186,194,202),NCCHANNEL_INITIALIZER(98,108,118),NCCHANNEL_INITIALIZER(98,108,118));
 struct ncvisual*v=ncvisual_from_rgba(rgba,864,936*4,936);free(rgba);
 struct ncvisual_options vo={.n=std,.y=2,.x=88,.scaling=NCSCALE_NONE,.blitter=NCBLIT_PIXEL,.flags=NCVISUAL_OPTION_CHILDPLANE|NCVISUAL_OPTION_NODEGRADE};
 struct ncplane*bitmap=ncvisual_blit(nc,v,&vo);
 ncplane_options to={.y=0,.x=0,.rows=40,.cols=175};struct ncplane*text=ncplane_create(std,&to);
 uint64_t transparent=0;ncchannels_set_bg_alpha(&transparent,NCALPHA_TRANSPARENT);ncchannels_set_fg_rgb(&transparent,0x121c26);
 uint64_t base=transparent;ncchannels_set_fg_alpha(&base,NCALPHA_TRANSPARENT);
 ncplane_set_base(text,"",0,base);ncplane_set_channels(text,transparent);
 ncplane_putstr_yx(text,3,4,"NOTCURSES CELL GRADIENT");ncplane_putstr_yx(text,3,90,"SAME AUTHORED PIXEL MATERIAL");
 ncplane_putstr_yx(text,6,4,"THRESHOLD    -18.0 dBFS");ncplane_putstr_yx(text,6,90,"THRESHOLD    -18.0 dBFS");
 ncplane_putstr_yx(text,23,4,"HALF-CELL GRADIENT / 2x1");
 ncplane_putstr_yx(text,8,90,bitmap?"PIXEL BLITTER AVAILABLE":"PIXEL BLITTER REFUSED");
 notcurses_render(nc);
 FILE*stats=fopen(argv[2],"w");if(!stats){notcurses_stop(nc);return 6;}
 fprintf(stats,"C %s pixel %d\n",notcurses_version(),bitmap!=NULL);
 ncstats*s=notcurses_stats_alloc(nc);
 for(int mode=0;mode<2;mode++){
  notcurses_stats_reset(nc,s);double begin=now();
  for(int i=0;i<100;i++){
   if(mode)ncplane_printf_yx(text,10,90,"RATIO        %2d:1",1+i%20);
   notcurses_render(nc);
  }
  notcurses_stats(nc,s);fprintf(stats,"%s 100 renders %.3f ms bytes %lu\n",mode?"changing":"unchanged",(now()-begin)*1000,(unsigned long)s->raster_bytes);
 }
 fclose(stats);free(s);sleep(8);ncvisual_destroy(v);return notcurses_stop(nc);
}
