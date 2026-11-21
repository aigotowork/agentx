/* Single presentation clock; selects exported checkpoints only. */
(function (global) {
  'use strict';
  const durations = {call:1800,effect:1500,reply:1800,outcome:1300,terminal:900,initial:900};
  /** One RAF clock. Index changes only at the exported checkpoint application boundary. */
  function create({onChange}) {
    let scenario=null,index=0,elapsed=0,speed=0.5,playing=false,frame=null,last=null;
    function state(){const target=scenario?.checkpoints[index+1]??null;const duration=durations[target?.phase]??900;return {scenarioId:scenario?.id??null,index,checkpoint:scenario?.checkpoints[index]??null,target,progress:Math.min(1,elapsed/duration),elapsed,speed,playing,phase:target&&elapsed>0?target.phase:'idle'};}
    function emit(applied=false){onChange(state(),applied);}
    function stopFrame(){if(frame!==null)cancelAnimationFrame(frame);frame=null;last=null;}
    function schedule(){if(playing&&frame===null)frame=requestAnimationFrame(tick);}
    function tick(now){frame=null;if(!playing)return;if(last!==null)elapsed+=Math.max(0,Math.min(100,now-last))*speed;last=now;
      const target=scenario?.checkpoints[index+1];
      if(!target){playing=false;stopFrame();emit();return;}
      if(elapsed>=(durations[target.phase]??900)){index+=1;elapsed=0;if(index>=scenario.checkpoints.length-1){playing=false;stopFrame();}emit(true);}else emit();schedule();
    }
    function pause(){playing=false;stopFrame();emit();}
    function seek(next){pause();index=Math.max(0,Math.min(scenario?scenario.checkpoints.length-1:0,next));elapsed=0;emit(true);}
    return Object.freeze({
      context(next){stopFrame();playing=false;scenario=next;index=0;elapsed=0;emit(true);},
      play(){if(!scenario||index>=scenario.checkpoints.length-1)return;playing=true;last=null;emit();schedule();},
      pause,next(){seek(index+1);},previous(){seek(index-1);},seek,
      replay(){seek(0);this.play();},
      speed(value){if(![0.25,0.5,1,2].includes(value))return;speed=value;last=null;emit();},
      state,dispose(){playing=false;stopFrame();}
    });
  }
  global.WorkbenchPlayer = Object.freeze({create});
}(globalThis));
