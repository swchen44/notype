import { useRef, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { commands, type ConnectionActivityView, type HistoryUsageDto } from '@/bindings';
import { connectionConfigurationChanged } from '@/hooks/useConnectionActivity';
import { Button } from '../../ui/Button';

export function ConnectionActivity({id,revision,activity,refresh}:{id:string;revision:number;activity?:ConnectionActivityView;refresh:()=>Promise<void>}) {
  activity=activity?.configuration_revision===revision?activity:undefined;
  const {t}=useTranslation();
  const [busy,setBusy]=useState(false),[message,setMessage]=useState('');
  const lock=useRef(false);
  const run=async(test:boolean)=>{
    if(lock.current)return;lock.current=true;setBusy(true);setMessage('');
    try{
      const result=test?await commands.testTextConnection(revision,id):await commands.clearConnectionUsage(id);
      setMessage(t(result.status==='ok'?(test?'notype.usability.testOk':'notype.usability.usageCleared'):'notype.usability.testFailed'));
    }catch{setMessage(t('notype.usability.testFailed'));}
    finally{lock.current=false;setBusy(false);await refresh();connectionConfigurationChanged();}
  };
  const count=(value?:HistoryUsageDto)=>value?.status==='complete'?String(value.total??0):value?.known_subtotal!==null && value?.known_subtotal!==undefined?`${value.known_subtotal} + ${t('notype.metrics.unknown')}`:t('notype.metrics.unknown');
  const fees=activity?.summary.fees;
  return <div className="space-y-2 text-sm">
    <p>{t('notype.usability.tokenInput')}: {count(activity?.summary.input)} · {t('notype.usability.tokenOutput')}: {count(activity?.summary.output)}</p>
    <p>{t('notype.usability.actualFees')}: {fees?.status==='complete' && !fees.totals.length?'0':(fees?.status==='complete'?fees.totals:fees?.known_subtotals)?.map(v=>`${v.currency} ${v.amount}`).join(' + ') || t('notype.metrics.unknown')}{fees?.status==='partial'?` + ${t('notype.metrics.unknown')}`:''}</p>
    <p>{t(activity?.test_success===true?'notype.usability.testOk':activity?.test_success===false?'notype.usability.testFailed':'notype.usability.notTested')}{activity?.tested_at_ms?` · ${new Date(activity.tested_at_ms).toLocaleString()}`:''}</p>
    {activity?.test_error && <p role="note">{t(`notype.errors.${activity.test_error}`,{defaultValue:t('notype.usability.testFailed')})}</p>}
    <div className="flex gap-2"><Button size="sm" disabled={busy} onClick={()=>void run(true)}>{t(busy?'notype.usability.testing':'notype.usability.testConnection')}</Button><Button size="sm" variant="secondary" disabled={busy} onClick={()=>void run(false)}>{t('notype.usability.clearUsage')}</Button></div>
    <p className="text-xs text-text/60">{t('notype.usability.testCost')}</p><p role="status">{message}</p>
  </div>;
}
