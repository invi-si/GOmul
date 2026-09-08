"""Explicit, reversible macOS AVD activation; no priority/governor overrides."""
import json, subprocess, time

def activate(pid):
    pid=int(pid)
    if pid <= 0: raise ValueError('Expected running AVD host PID')
    js=f'''ObjC.import("AppKit"); var app=$.NSRunningApplication.runningApplicationWithProcessIdentifier({pid}); app.unhide; var ok=app.activateWithOptions(3); JSON.stringify({{activated:!!ok,pid:{pid}}});'''
    result=json.loads(subprocess.check_output(['osascript','-l','JavaScript','-e',js],text=True))
    if not result['activated']: raise RuntimeError('AVD activation failed')
    time.sleep(0.25)  # Let the host process the activation event, outside timing.
    result['threads']=subprocess.check_output(['ps','-M','-p',str(pid)],text=True)
    result['frontmost_pid']=int(subprocess.check_output(['osascript','-l','JavaScript','-e','ObjC.import("AppKit"); $.NSWorkspace.sharedWorkspace.frontmostApplication.processIdentifier;'],text=True).strip())
    return result
