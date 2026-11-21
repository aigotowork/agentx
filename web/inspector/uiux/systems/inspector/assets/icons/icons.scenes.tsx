/** @aiPurpose Reuse actual semantic host icons for a compact composition and exhaustive inspection. */
import {Icon} from '../../platforms/host.web';import {pixelIconPaths} from './icons.paths';
function Icons({compact=false}:{compact?:boolean}){return <div className="asset-icons">{(compact?['Search','Folder','Layers','Play','RotateCcw','Settings2','ChartColumn','TriangleAlert','Check','CircleHelp']:Object.keys(pixelIconPaths)).map(name=><div key={name} title={name}><Icon name={name}/>{!compact&&<span>{name}</span>}</div>)}</div>;}
/** @aiPurpose Provide scenes presentation behavior.
 * @aiUsage Use within the Inspector host and its themed resource boundary. */
export const scenes={default:()=> <Icons/>,compact:()=> <Icons compact/>};
