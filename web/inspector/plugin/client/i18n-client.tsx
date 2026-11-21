/** Local catalog hooks; native settings/event acquisition is intentionally deferred. */
import {useSyncExternalStore} from 'react';
import {onTextChange,t,translator} from '../shared/i18n';
export function deviceLanguage():string{return navigator.language;}
/** @aiPurpose Subscribe presentation to the local catalog.
 * @aiUsage Use within Inspector; language settings change this shared translator. */
export function useI18n(){const language=useSyncExternalStore(onTextChange,()=>translator().language,()=>translator().language);return {t,language};}
