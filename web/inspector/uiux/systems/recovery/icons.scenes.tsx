/** @aiPurpose Preview the actual maintained legacy vectors, keeping CSS and semantics isolated. */
import {Icon,iconNames} from './recovery.web';import {RecoveryPreview} from './recovery.preview';export const scenes={default:()=> <RecoveryPreview><div className="preview-row">{iconNames.map(name=><span title={name} key={name}><Icon name={name}/></span>)}</div></RecoveryPreview>};
