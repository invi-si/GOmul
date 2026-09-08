// Full Android-library replay driver. Input: /data/local/tmp/wie-cpu-replay.json.
// Reset and complete-state validation are outside each timed execution window.
#include <dlfcn.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <time.h>
static uint64_t now_ns(void) {struct timespec t; clock_gettime(CLOCK_MONOTONIC,&t);return (uint64_t)t.tv_sec*1000000000ull+t.tv_nsec;}
static void *symbol(void *lib,const char *name) {void *p=dlsym(lib,name);if(!p){fprintf(stderr,"%s\n",dlerror());exit(2);}return p;}
int main(int argc,char **argv) {
 if(argc<2||argc>3)return 2;
 unsigned iterations=argc==3?strtoul(argv[2],NULL,10):1000;
 if(!iterations)return 2;
 void *lib=dlopen(argv[1],RTLD_NOW|RTLD_LOCAL);if(!lib){fprintf(stderr,"%s\n",dlerror());return 2;}
 uint32_t (*setup)(void)=symbol(lib,"wie_replay_setup");
 uint32_t (*reset)(void)=symbol(lib,"wie_replay_reset");
 uint32_t (*run)(void)=symbol(lib,"wie_replay_run");
 uint32_t (*validate)(void)=symbol(lib,"wie_replay_validate");
 uint32_t (*instructions)(void)=symbol(lib,"wie_replay_instructions");
 if(setup())return 3;
 for(unsigned i=0;i<50;i++)if(reset()||run()||validate())return 4;
 uint64_t elapsed=0,clock_pair=0;
 for(unsigned i=0;i<iterations;i++){
  if(reset())return 4;
  uint64_t begin=now_ns(); unsigned error=run(); elapsed+=now_ns()-begin;
  if(error||validate()){fprintf(stderr,"Replay validation failed at iteration %u\n",i);return 4;}
 }
 for(unsigned i=0;i<iterations;i++){uint64_t start=now_ns();clock_pair+=now_ns()-start;}
 printf("{\"iterations\":%u,\"timedNanoseconds\":%llu,\"clockPairNanoseconds\":%llu,\"instructionsPerRun\":%u,\"validated\":true}\n",iterations,(unsigned long long)elapsed,(unsigned long long)clock_pair,instructions());
 return 0;
}
