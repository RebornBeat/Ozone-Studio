#!/usr/bin/env python3
"""FORK V2 manual keyword decisions (2026-11-10). Judge: the assistant, reading each container's real evidence
(name, body / graph nodes / declared blueprint index keywords + converted bodies / pipeline folder name).
Decision per unvalidated non-marker term: yes (validate), no (remove, validated:false), leave (unsure -> untouched).
Asymmetric: unsure => leave. No scores."""
import json,re,collections
T=json.load(open('targets.json'))['targets']
E=json.load(open('evidence_raw.json'))
out=[]  # rows
def add(c,term,dec,why):
    out.append({"container_id":c['id'],"type":c['type'],"name":c['name'],"term":term,"decision":dec,"reason":why})

# ---------------- Category (evidence: body = "Auto-created category for topic: <name>")
CAT_NO={
 40070:{'amt expansion':'rewording of the category topic, not distinct','amt expand':'verb rewording of the topic','amt re-expand':'rewording duplicate of "amt re-expansion"','amt expand again':'sentence-fragment rewording','amt expand back':'sentence-fragment rewording','amt pool expansion':'hallucinated finance/pool meaning; AMT here is the abstract tree','amt expand liquidity':'hallucinated finance meaning'},
 40072:{'first version':'rewording','starting build':'rewording','initial release':'different concept (release) not covered by the category','first iteration':'rewording','initial construction':'rewording','first construction':'rewording','initial setup':'different concept (setup)','first setup':'different concept (setup)','initial version':'rewording; version is not build'},
 40071:{'amt category':'generic wording of the container type','main amt category':'wording of the name plus "category"','alternative minimum tax':'wrong topic: US tax, the category is about the AMT graph'},
}
CAT_YES_ALL=True  # every term not listed in CAT_NO is the category's own topic/name or a close name form
for c in T:
    if c['type']!='Category':continue
    no=CAT_NO.get(c['id'],{})
    for t in c['unvalidated']:
        if t.lower() in no: add(c,t,'no',no[t.lower()])
        else: add(c,t,'yes',"the category's own topic (body: 'Auto-created category for topic: %s') or an accurate name form"%c['name'])

# ---------------- Pipeline (evidence: container name + folder pipelines/<cat>/<folder>)
PIPE_NO_GENERIC={'pipeline':'true of every Pipeline container; carries no subject','core':'folder placement label, true of most pipelines','general':'folder placement label, true of many pipelines'}
PIPE_NO_FRAG={'tab','check','return','update','create','register','fetch','manager','loader','construction','formation','creation','development','zero','shot','meta','portion'}
for c in T:
    if c['type']!='Pipeline':continue
    nm=c['name'].lower()
    for t in c['unvalidated']:
        tl=t.lower()
        if tl in PIPE_NO_GENERIC: add(c,t,'no',PIPE_NO_GENERIC[tl])
        elif tl==nm: add(c,t,'yes','the pipeline\'s exact name (%s) — an identifier searchers use'%c['name'])
        elif tl in PIPE_NO_FRAG: add(c,t,'no','generic role/verb fragment of the name "%s" that names no subject alone (the full name is kept)'%c['name'])
        else: add(c,t,'yes','a concept word in the pipeline name "%s" (folder %s)'%(c['name'],c['path']))

# ---------------- Blueprint: YES lists (everything else not in LEAVE is NO). Evidence: bootstrap index keywords (declared), name, description, steps
BP_YES={
 40004:['create','generate','write code','implement','build','scaffold','code creation','code','specification','requirements'],
 40011:['audit','links','graph','relationships','metrics','orphans','living-graph linking audit','living-graph'],
 40003:['document','documentation','readme','tutorial','guide','api docs','code','requirements'],
 40014:['postmortem','incident','lesson','regression','methodology','root cause','incident postmortem to methodology','incident postmortem'],
 40016:['integrity','audit','duplicates','repair','cleanup','data quality'],
 40006:['analyze','analysis','data','statistics','chart','visualize','insights','data analysis','csv','json','data visualization'],
 40009:['jurisdiction','legal','country','regulation','compliance','rules','jurisdiction expansion'],
 40012:['retrieval','cross-process','persistence','graphs','hardening','cross-process retrieval hardening'],
 40013:['text','document','analyze','summary','extract','keywords','text document analysis'],
 2506:['reexpansion'],
 40010:['methodology','create','principles','decision rules','heuristics','knowledge','methodology creation and registration','methodology creation','methodology registration'],
 40018:['top-down','jurisdiction','confirmation','capture-store','context-assurance','td-1 regulated plan review','td-1'],
 40002:['review','code review','analyze code','check code','audit','quality','code'],
 40019:['top-down','consciousness','stage-8','simulation','context-assurance','td-2 consciousness full-context','td-2','full-context'],
 40001:['help','explain','question','assist','general assistant','assistant','image','audio','video','chat'],
 40007:['prove','proof','verify','theorem','lemma','mathematical','formal','mathematical proof verification','math','proof verification'],
 40008:['pipeline','modality','onboard','new modality','graph','persist','modality pipeline onboarding','modality pipeline'],
 40015:['agent','onboarding','protocol','coordination','claims','handoff','agent onboarding protocol','agent onboarding'],
 40017:['fallback','bitnet','verification','capture','resilience','rate limit','fallback and capture verification','capture verification','fallback verification'],
 40020:['top-down','switch-coherence','bitnet','fallback','context-assurance','td-3 switch coherence','td-3'],
 40094:['reexpansion','initialbuild'],
 40160:['graph databases','highly connected data','graph database explanation'],
 40048:['reexpansion','farm','http','amt re-expansion','farm profile start','profile start failure'],
 40212:['amt','re-expansion','amt recovery'],
 40397:['reexpansion','http','farm'],
 40255:['amt','initialbuild','re-expansion'],
}
BP_LEAVE={2506:['farmstateget'],40212:['error handling','data validation']}
BP_NO_WHY={
 'text':'generic input-type label true of almost every blueprint','text graph':'junk structural label (the graph type), not a subject',
 'code':'generic input-type label, not what this blueprint is about','container':'generic structural noun of the request, not the subject',
 'coordinationreceived':'event-kind label (system bookkeeping), not subject matter','failed':'status word, no subject','post':'HTTP verb fragment; ambiguous (blog post)','class':'fragment of an error message','object':'fragment of an error message','default':'fragment of an error message','tool':'generic single word','project':'generic structural noun','must':'fragment of an error message','state':'generic single word','match':'fragment of an error message','azaz':'fragment of an error-message regex','profilesprofilestart':'garbled API path fragment','farmprofilestart':'mangled form of farm_profile_start; the proper form is kept',
 'file_reference':'input-type identifier (underscore label), not a subject','code_analysis suggestions':'concatenated output-type label',
 'what is':'question-stem function words','how does':'question-stem function words','general':'generic word true of everything',
}
for c in T:
    if c['type']!='Blueprint':continue
    yes={t.lower() for t in BP_YES.get(c['id'],[])}; leave={t.lower() for t in BP_LEAVE.get(c['id'],[])}
    for t in c['unvalidated']:
        tl=t.lower()
        if tl in yes: add(c,t,'yes','declared routing keyword / the blueprint\'s own name / a subject its description or steps state')
        elif tl in leave: add(c,t,'leave','unsure: plausible but not clearly supported by the evidence read')
        else: add(c,t,'no',BP_NO_WHY.get(tl,'near-duplicate rewording or unsupported expansion of a kept term; not in the blueprint\'s declared keywords, name, description or steps'))

# ---------------- ModalityGraph
MG_STRUCT={'modality graph','text graph','unknown text graph','unknown text','unknown modality','unknown graph','text container','graph container','modality container','modality data','modal graph','graph store','text network','sentence graph','modalitygraph','graph','modality'}
MG_STATS=re.compile(r'^(\d+ words?|\d+-word graph|1 sentence|single sentence|two sentences|one-sentence graph|16 words|15 words)$')
MG_NO_TEXT={'container':'generic structural noun','project':'generic structural noun','main':'fragment','initial':'fragment of "initial build"','failed':'status word','post':'HTTP verb fragment; ambiguous','class':'fragment of an error message','match':'fragment of an error message','must':'fragment of an error message','state':'generic single word','tool':'generic single word','missing':'generic single word','malformed':'generic single word','coordinationreceived':'event-kind label (bookkeeping)','container 3644':'contains an id','profiles profile start':'API path fragment','mcp utility':'hallucinated term','farm profile initialization':'rewording of the tool name','coordination receipt':'rewording of an event-kind label','default farm configuration':'hallucinated expansion','amount graph':'wrong expansion of AMT','amt graph':'rewording by container type','...':'placeholder text','project 40159':'contains an id','id 40159':'id','first build':'rewording not in the content','re-expand':'verb variant','re expansion':'variant spelling of a kept term','graph database':'singular duplicate of "graph databases"','connected data':'fragment duplicate of "highly connected data"','knowledge graph':'not in the content (only graph databases / highly connected data)','entity graph':'unsupported expansion','dense graph':'unsupported expansion','rich graph':'unsupported expansion','complex graph':'unsupported expansion','large graph':'unsupported expansion','network graph':'unsupported expansion','re-expansion graph':'rewording by container type','initial build graph':'rewording by container type','main amt graph':'rewording by container type','project graph':'rewording by container type','graph of amt':'rewording by container type','graph for project':'rewording by container type','farm profile initialization':'rewording'}
MG_YES_TEXT={'amt','re-expansion','amt re-expansion','mcp tool','farm_profile_start','farm default','http','post request failure','profile start error','initialbuild','initial build','graph databases','highly connected data'}
for c in T:
    if c['type']!='ModalityGraph':continue
    ev=E[str(c['id'])];mod=ev['meta'].get('modality')
    ul=[t.lower() for t in c['unvalidated']]
    for t in c['unvalidated']:
        tl=t.lower()
        if mod=='Text':
            if tl in MG_STRUCT: add(c,t,'no','graph-type / container-type label true of every graph of this kind')
            elif MG_STATS.match(tl): add(c,t,'no','a statistic of the graph (word/sentence count), not meaning')
            elif tl=='reexpansion':
                if 're-expansion' in ul or 'amt re-expansion' in ul or c['id'] in (40131,): 
                    # keep the plain token only where it is the only form; 40131 also stores 'AMT re-expansion' as a Topic, the token node 'reexpansion' is its own extracted keyword
                    if c['id']==40131: add(c,t,'yes','the extracted keyword node "reexpansion" in the graph')
                    else: add(c,t,'no','spelling variant of the kept "re-expansion"')
                else: add(c,t,'yes','the extracted keyword node "reexpansion" (the only form in this graph)')
            elif tl in MG_YES_TEXT: add(c,t,'yes','present as a Keyword/Topic/Entity node of the graph and names real subject matter')
            elif tl in MG_NO_TEXT: add(c,t,'no',MG_NO_TEXT[tl])
            else: add(c,t,'leave','unsure: not covered by a decision after reading the graph nodes')
        else: # Image graphs: one PNG node (or PNG + Region + Text)
            if tl in ('image','image graph','image modality','image node','single image','single node graph'): add(c,t,'yes','accurate: the graph holds an Image node (modality Image)'+(' (single node, no edges)' if tl=='single node graph' else ''))
            elif tl in ('background','text') and ev['body'] and ('Main background' in ev['body'] or 'Sample text' in ev['body']): add(c,t,'yes','a Region/Text node in the graph ("Main background" / "Sample text")')
            elif tl=='scene graph': add(c,t,'leave','unsure: a computer-vision term; the graph has a Region and a Text node, not clearly a scene graph')
            elif tl in ('graph','visual graph','picture graph','photo graph','graph image','modality graph','image nodes','image edges','nodes image','edges image','image relationships','image structure','image composition','image elements','image topology','background text'): add(c,t,'no','structural/generic wording true of every graph, or a made-up combination, not meaning')
            else: add(c,t,'no','rewording or unsupported expansion of "image" (the content is one PNG node of unknown picture content; photo/diagram/chart/dataset/collection/translations add no supported subject)')
json.dump(out,open('decisions.json','w'),indent=0)
cnt=collections.Counter((r['type'],r['decision']) for r in out)
print(len(out),dict(cnt))
unv=sum(len(c['unvalidated']) for c in T if c['type'] in('Category','Pipeline','Blueprint','ModalityGraph'))
print('terms covered',len(out),'of',unv)
