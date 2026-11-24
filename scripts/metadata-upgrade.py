"""Read Appterisk metadata only. Never access browser authentication databases."""
import hashlib,json,os,sqlite3,sys
from pathlib import Path
root=Path(os.environ['LOCALAPPDATA'])/'com.appterisk.messenger'
connection=sqlite3.connect((root/'appterisk.sqlite3').as_uri()+'?mode=ro',uri=True)
accounts=connection.execute('SELECT id,name,platform,profile_key,icon,display_order,notifications_enabled,created_at,updated_at FROM accounts ORDER BY id').fetchall()
settings=connection.execute('SELECT key,value FROM application_settings ORDER BY key').fetchall()
# Last-active selection may legitimately change at startup if a default account is configured.
normalized=[]
for key,value in settings:
    if key=='preferences':
        parsed=json.loads(value);parsed.pop('last_active_account',None);value=json.dumps(parsed,sort_keys=True)
    normalized.append((key,value))
permissions=connection.execute('SELECT account_id,origin,kind,decision FROM origin_permissions ORDER BY account_id,origin,kind').fetchall()
def digest(value):return hashlib.sha256(json.dumps(value,sort_keys=True).encode()).hexdigest()
report={'schema':connection.execute('PRAGMA user_version').fetchone()[0],'account_count':len(accounts),'accounts_sha256':digest(accounts),'settings_sha256':digest(normalized),'permissions_sha256':digest(permissions),'browser_databases_read':False}
connection.close()
destination=Path(__file__).resolve().parent.parent/'docs'/'release-evidence-0.4.0'
destination.mkdir(parents=True,exist_ok=True)
if sys.argv[1]=='before':
    (destination/'metadata-before.json').write_text(json.dumps(report,indent=2)+'\n')
else:
    before=json.loads((destination/'metadata-before.json').read_text())
    for key in ['account_count','accounts_sha256','settings_sha256','permissions_sha256']:
        if report[key]!=before[key]:raise RuntimeError('Metadata preservation check failed: '+key)
    if before['schema']!=2 or report['schema']!=3:raise RuntimeError('Expected schema 2 to 3 migration')
    report['preserved']=True
    (destination/'metadata-after.json').write_text(json.dumps(report,indent=2)+'\n')
print(json.dumps(report))
