import { useTranslation } from 'react-i18next';
import { useSettings } from '@/hooks/useSettings';
import { useConnectionActivity } from '@/hooks/useConnectionActivity';

export function ConnectionStatus() {
  const {t}=useTranslation();
  const {getSetting}=useSettings();
  const {configuration,activity,failed}=useConnectionActivity();
  const enabled=getSetting('post_process_enabled')??false;
  const selected=configuration?.connections.find(c=>c.id===configuration.active_connection_id);
  const status=activity.find(c=>c.connection_id===selected?.id);
  return <section className="mt-auto w-full border-t border-mid-gray/20 p-2 space-y-1 text-xs break-words" aria-label={t('notype.usability.connectionStatus')}>
    <p className="font-medium">{t(enabled?'notype.usability.aiOn':'notype.usability.aiOff')}</p>
    <p>{selected?.display_name??t('notype.connections.noActive')}</p>
    {selected && <p className="break-all text-text/60" title={selected.effective_endpoint??selected.api_address}>{selected.effective_endpoint??selected.api_address}</p>}
    {selected && <p>{selected.model}</p>}
    <p role="status">{t(failed?'notype.usability.activityUnavailable':status?.test_success===true?'notype.usability.testOk':status?.test_success===false?'notype.usability.testFailed':'notype.usability.notTested')}</p>
  </section>;
}
