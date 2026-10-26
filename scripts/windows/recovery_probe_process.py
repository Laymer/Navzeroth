"""Own only freshly created process trees; no enumeration/control of game processes."""
import os
import signal
import subprocess


class WindowsJob:
    """Suspend our new child, assign a kill-on-close job, then resume its thread."""
    def __init__(self):
        import ctypes as c
        from ctypes import wintypes as w
        self.c, self.w = c, w
        self.api = c.WinDLL('kernel32', use_last_error=True)
        class Limits(c.Structure):
            _fields_ = [('process_time', c.c_longlong), ('job_time', c.c_longlong),
                        ('flags', w.DWORD), ('minimum', c.c_size_t), ('maximum', c.c_size_t),
                        ('processes', w.DWORD), ('affinity', c.c_size_t),
                        ('priority', w.DWORD), ('scheduling', w.DWORD)]
        class Counters(c.Structure):
            _fields_ = [(name, c.c_ulonglong) for name in ('reads','writes','other','read_bytes','write_bytes','other_bytes')]
        class Extended(c.Structure):
            _fields_ = [('basic', Limits), ('io', Counters)] + [(name, c.c_size_t) for name in ('process_memory','job_memory','peak_process','peak_job')]
        class Accounting(c.Structure):
            _fields_ = [(name, c.c_longlong) for name in ('user','kernel','period_user','period_kernel')] + [(name, w.DWORD) for name in ('faults','total','active','terminated')]
        class Thread(c.Structure):
            _fields_ = [('size', w.DWORD), ('usage', w.DWORD), ('id', w.DWORD), ('owner', w.DWORD), ('base', w.LONG), ('delta', w.LONG), ('flags', w.DWORD)]
        self.Accounting, self.Thread = Accounting, Thread
        functions = {
            'CreateJobObjectW': ([c.c_void_p,w.LPCWSTR],w.HANDLE),
            'SetInformationJobObject': ([w.HANDLE,c.c_int,c.c_void_p,w.DWORD],w.BOOL),
            'AssignProcessToJobObject': ([w.HANDLE,w.HANDLE],w.BOOL),
            'QueryInformationJobObject': ([w.HANDLE,c.c_int,c.c_void_p,w.DWORD,c.c_void_p],w.BOOL),
            'TerminateJobObject': ([w.HANDLE,w.UINT],w.BOOL),
            'CloseHandle': ([w.HANDLE],w.BOOL),
            'OpenProcess': ([w.DWORD,w.BOOL,w.DWORD],w.HANDLE),
            'OpenThread': ([w.DWORD,w.BOOL,w.DWORD],w.HANDLE),
            'ResumeThread': ([w.HANDLE],w.DWORD),
            'CreateToolhelp32Snapshot': ([w.DWORD,w.DWORD],w.HANDLE),
            'Thread32First': ([w.HANDLE,c.POINTER(Thread)],w.BOOL),
            'Thread32Next': ([w.HANDLE,c.POINTER(Thread)],w.BOOL),
        }
        for name,(args,result) in functions.items():
            function=getattr(self.api,name);function.argtypes=args;function.restype=result
        self.handle=self.api.CreateJobObjectW(None,None)
        if not self.handle:
            raise c.WinError(c.get_last_error())
        limits=Extended();limits.basic.flags=0x2000  # JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE
        if not self.api.SetInformationJobObject(self.handle,9,c.byref(limits),c.sizeof(limits)):
            error=c.get_last_error()
            self.close()
            raise c.WinError(error)

    def assign_and_resume(self, pid):
        c=self.c
        process=self.api.OpenProcess(0x0100|0x0001,False,pid)  # SET_QUOTA | TERMINATE, own suspended PID
        if not process:
            raise c.WinError(c.get_last_error())
        try:
            if not self.api.AssignProcessToJobObject(self.handle,process):
                raise c.WinError(c.get_last_error())
        finally:
            self.api.CloseHandle(process)
        snapshot=self.api.CreateToolhelp32Snapshot(0x00000004,0)  # TH32CS_SNAPTHREAD
        if snapshot == c.c_void_p(-1).value:
            raise c.WinError(c.get_last_error())
        try:
            entry=self.Thread();entry.size=c.sizeof(entry)
            found=self.api.Thread32First(snapshot,c.byref(entry))
            while found:
                if entry.owner == pid:
                    thread=self.api.OpenThread(0x0002,False,entry.id)  # SUSPEND_RESUME, own child only
                    if not thread:
                        raise c.WinError(c.get_last_error())
                    try:
                        previous=self.api.ResumeThread(thread)
                        if previous != 1:
                            raise OSError('Own primary thread could not be resumed from suspended creation')
                        return
                    finally:
                        self.api.CloseHandle(thread)
                found=self.api.Thread32Next(snapshot,c.byref(entry))
            raise OSError('Own suspended primary thread was not found')
        finally:
            self.api.CloseHandle(snapshot)

    def active(self):
        value=self.Accounting()
        if not self.api.QueryInformationJobObject(self.handle,1,self.c.byref(value),self.c.sizeof(value),None):
            raise self.c.WinError(self.c.get_last_error())
        return value.active > 0

    def kill(self):
        if self.handle and not self.api.TerminateJobObject(self.handle,137):
            raise self.c.WinError(self.c.get_last_error())

    def close(self):
        if self.handle:
            self.api.CloseHandle(self.handle)
            self.handle=None


class OwnedTree:
    def __init__(self, command, env=None, cwd=None):
        self.job=WindowsJob() if os.name == 'nt' else None
        self.process=None
        try:
            self.process=subprocess.Popen(command,stdout=subprocess.PIPE,stderr=subprocess.PIPE,
                stdin=subprocess.DEVNULL,env=env,cwd=cwd,start_new_session=os.name != 'nt',
                creationflags=0x00000004 if os.name == 'nt' else 0)  # CREATE_SUSPENDED
            if self.job:
                self.job.assign_and_resume(self.process.pid)
        except BaseException:
            if self.process is not None:
                self.process.kill();self.process.wait(timeout=5)
                self.process.stdout.close();self.process.stderr.close()
            if self.job:
                self.job.close()
            raise

    def active(self):
        if self.job:
            return self.job.active()
        try:
            os.killpg(self.process.pid,0)
            return True
        except ProcessLookupError:
            return False

    def kill(self):
        if self.job:
            self.job.kill()
        else:
            try:
                os.killpg(self.process.pid,signal.SIGKILL)
            except ProcessLookupError:
                pass
        if self.process.poll() is None:
            self.process.kill()

    def close(self):
        if self.job:
            self.job.close()  # Also kills any owned descendant surviving parent exit.
