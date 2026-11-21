/** @aiPurpose Render actual owned resources for inspection without product effects. */
import {space,fontSize,pixelShape,pixelMotion} from '../inspector.tokens';
/** @aiPurpose Provide scenes presentation behavior.
 * @aiUsage Use within the Inspector host and its themed resource boundary. */
export const scenes={default:()=> <div className="token-records">{Object.entries({space,fontSize,shape:pixelShape,motion:pixelMotion}).map(([group,values])=><section key={group}><h3>{group}</h3>{Object.entries(values).map(([key,value])=><div key={key}><code>{key}</code><output>{value}</output></div>)}</section>)}</div>};
