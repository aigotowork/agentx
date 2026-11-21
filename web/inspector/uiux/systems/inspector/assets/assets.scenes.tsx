/** @aiPurpose Render actual owned resources for inspection without product effects. */
import {assetRoles,resolveAsset} from './assets.roles';
/** @aiPurpose Provide scenes presentation behavior.
 * @aiUsage Use within the Inspector host and its themed resource boundary. */
export const scenes={default:()=> <div className="asset-gallery">{Object.entries(assetRoles).map(([role,meta])=><figure key={role}>{meta.kind==='svg'?<img src={resolveAsset(role as keyof typeof assetRoles)} alt={role}/>:<div className="asset-texture" data-kind={meta.kind}/>}<figcaption><code>{role}</code><p>{meta.origin} · {meta.license}</p></figcaption></figure>)}</div>};
