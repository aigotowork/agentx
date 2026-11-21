/** @aiPurpose Render actual owned resources for inspection without product effects. */
import {useDesignSystem} from '../platforms/web/studio.context';
/** @aiPurpose Provide scenes presentation behavior.
 * @aiUsage Use within the Inspector host and its themed resource boundary. */
export const scenes={default:()=>{const {resolved}=useDesignSystem();return <div className="token-records">{Object.entries(resolved.variables).map(([key,value])=><div key={key}><code>{key}</code><output>{value}</output></div>)}</div>;}};
