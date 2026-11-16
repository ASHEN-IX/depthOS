"""Pure fail-closed checks used before native host input/capture."""
KEYS=frozenset({1,3,11,12,13,14,18,20,25,28,31,32,33,35,37,42,57,103,105,106,108})
def valid_command(text):
 parts=text.split()
 if len(parts)==3 and parts[0]=='key':return parts[1].isdigit() and int(parts[1]) in KEYS and parts[2] in ['0','1']
 if len(parts)==3 and parts[0]=='button':return parts[1]=='272' and parts[2] in ['0','1']
 if len(parts)==3 and parts[0]=='move':
  try:return 0<=float(parts[1])<1920 and 0<=float(parts[2])<1080
  except ValueError:return False
 return False

def native_capture_scope(state,context,current_desktop):
 if current_desktop!=context['desktop']:return False
 owned={w['native']:w['pid'] for w in context['windows']}
 background={w['native']:w['pid'] for w in context.get('backgroundWindows',[])}
 def owned_or_background(w):
  native=w['uuid'].strip('{}')
  return (native in owned and w['pid']==owned[native]) or (w.get('background',False) and native in background and w['pid']==background[native])
 visible=[w for w in state['windows'] if not w['desktops'] or context['desktop'] in w['desktops']]
 return all(owned_or_background(w) for w in visible) and (not state['active'] or state['active'].strip('{}') in owned)
