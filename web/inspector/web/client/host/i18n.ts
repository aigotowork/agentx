import { bindTranslator, createTranslator } from '../../../plugin/shared/i18n';
import zh from '../../../plugin/shared/locales/zh-CN.json';
import en from '../../../plugin/shared/locales/en.json';
import kitZh from '../../../plugin/shared/locales/kit/zh-CN.json';
import kitEn from '../../../plugin/shared/locales/kit/en.json';
const language = new URLSearchParams(location.search).get('language') === 'en' ? 'en' : 'zh-CN';
bindTranslator(createTranslator(language, language === 'en' ? [en, kitEn] : [zh, kitZh]));
document.documentElement.lang = language;
