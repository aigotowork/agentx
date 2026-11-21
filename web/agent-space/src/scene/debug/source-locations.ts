/** @aiPurpose Associate semantic Canvas observations with their real projection, rendering and behavior owners. */
import type {DebugObject} from './contracts';
const root='web/agent-space/';
const at=(file:string,line:number,name:string,kind:'definition'|'render'='definition')=>({file:root+file,line,column:0,name,kind});
/** These explicit source positions are maintained with their owning definitions, not inferred from snapshots or native instances. */
export function sceneObjectSources(object?:DebugObject){
 const sources=[at('src/scene/debug/objects.ts',17,'sceneObjects · semantic projection')];
 if(!object)return [at('src/scene/HallScene.tsx',33,'HallScene'),...sources];
 if('assetId'in object.details||object.details.art==='studio'){
   if(object.kind==='actor')return [...sources,at('src/scene/runtime/studio-render.ts',32,'Studio actor drawing','render'),at('src/scene/studio-art.ts',58,'studioActorArt'),at('src/scene/motion.ts',14,'sceneBehavior')];
   return [...sources,at('src/scene/runtime/studio-render.ts',10,'StudioPainter','render')];
 }
 if(object.kind==='actor')return [...sources,at('src/scene/runtime/renderer.ts',145,'drawActor','render'),at('src/scene/runtime/sprite.ts',8,'actorAnimation'),at('src/scene/motion.ts',14,'sceneBehavior')];
 if(object.kind==='human'||object.kind==='effect'&&'activity'in object.details)return [...sources,at('src/scene/runtime/renderer.ts',53,'decorative cat rendering','render'),at('src/scene/cat.ts',23,'sampleCat · decorative animation'),at('src/scene/runtime/renderer.ts',77,'human endpoint nest','render')];
 return [...sources,at('src/scene/runtime/renderer.ts',77,'SceneRenderer drawing','render')];
}
