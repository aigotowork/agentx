/** @aiPurpose Prove method removal and incompatible native input/output cannot silently remain source-bound.
 * @aiUsage Typecheck this file; negative declarations use required TypeScript errors and are never executed.
 */
import {bindNativeMethod} from '../tools/flow-atlas/domain/index.ts';
import {schema} from '../tools/flow-atlas/authoring/handles.ts';
import {zodAdapter} from '../tools/flow-atlas/authoring/adapters/zod.ts';
import {BeginInput,BeginResult,Order} from '../tools/flow-atlas/domain/examples/native.ts';
const input=schema(BeginInput,zodAdapter,{id:'input',label:'Input'});
const output=schema(BeginResult,zodAdapter,{id:'output',label:'Output'});
const spec={id:'method',label:'Begin',ownerId:'order',input,output};
bindNativeMethod({...spec,method:Order.prototype.beginReservation});
// @ts-expect-error Removal/renaming of a real public method breaks the reference.
bindNativeMethod({...spec,method:Order.prototype.removedBeginReservation});
bindNativeMethod({...spec,
  // @ts-expect-error Input must be the native Schema's intent contract.
  method:(_input:{quantity:number})=>({kind:'rejected' as const,reason:'wrong input'}),
});
bindNativeMethod({...spec,
  // @ts-expect-error Output must agree with the native result Schema.
  method:(_input:{intentKey:string})=>'not a BeginResult',
});
