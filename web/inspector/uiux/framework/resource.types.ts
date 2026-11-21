/** @aiPurpose Define serializable resource, state, interaction and experiment contracts independently of product UI. */
export type ResourceKind='foundation'|'theme'|'layout'|'component'|'block'|'asset'|'motion'|'sound'|'board';
export type ResourceControl={id:string;label:string;type:'boolean'|'text'|'select';default:string|boolean;options?:{label:string;value:string}[]};
export type ResourceScene={id:string;title:string;description:string;kind:'overview'|'state'|'interaction'|'experiment';controls?:ResourceControl[];question?:string;parts?:{id:string;title:string;resource:string;controls:string[]}[]};
export type ResourceState={id:string;title:string;description:string;scene:string};
export type ResourceInteraction={id:string;title:string;trigger:string;from:string;to:string;events:string[];scene:string;result:string;interruption:string};
export type Resource={schemaVersion:1;id:string;system:string;kind:ResourceKind;title:string;summary:string;maturity:'experimental'|'stable'|'deprecated';source:string[];design:string;useWhen:string[];avoidWhen:string[];dependencies:string[];events:{name:string;description:string}[];states:ResourceState[];interactions:ResourceInteraction[];scenes:ResourceScene[];usage:{entry:string;symbols:string[]};provenance?:{origin:string;license:string};roles?:string[]};
