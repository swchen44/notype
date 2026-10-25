// Own synthetic controls only. This helper never enumerates or reads other apps.
using System;
using System.Collections.Generic;
using System.Diagnostics;
using System.Runtime.InteropServices;
using System.Text;
using System.Threading;
using System.Web.Script.Serialization;
using System.Windows.Forms;
using System.Windows.Automation;

public class NativeInputFixture : Form {
  [DllImport("user32.dll",CharSet=CharSet.Unicode)] static extern IntPtr CreateWindowExW(int ex,string cls,string text,int style,int x,int y,int w,int h,IntPtr parent,IntPtr id,IntPtr instance,IntPtr arg);
  [DllImport("user32.dll")] static extern IntPtr SendMessageW(IntPtr h,uint m,IntPtr w,IntPtr l);
  [DllImport("user32.dll",EntryPoint="SendMessageW",CharSet=CharSet.Unicode)] static extern IntPtr SetText(IntPtr h,uint m,IntPtr w,string text);
  [DllImport("user32.dll",EntryPoint="SendMessageW")] static extern IntPtr Selection(IntPtr h,uint m,out uint start,out uint end);
  [DllImport("user32.dll",CharSet=CharSet.Unicode)] static extern int GetWindowTextW(IntPtr h,StringBuilder text,int max);
  [DllImport("user32.dll")] static extern IntPtr SetFocus(IntPtr h);
  [DllImport("user32.dll")] static extern bool SetForegroundWindow(IntPtr h);
  [DllImport("user32.dll")] static extern bool ShowWindow(IntPtr h,int command);
  [DllImport("user32.dll")] static extern bool BringWindowToTop(IntPtr h);
  [DllImport("user32.dll")] static extern bool IsWindowVisible(IntPtr h);
  [DllImport("user32.dll")] static extern IntPtr GetForegroundWindow();
  [DllImport("user32.dll")] static extern IntPtr GetFocus();
  [DllImport("user32.dll")] static extern uint GetWindowThreadProcessId(IntPtr h,out uint pid);
  [DllImport("kernel32.dll")] static extern uint GetCurrentThreadId();
  [DllImport("user32.dll")] static extern IntPtr GetThreadDesktop(uint thread);
  [DllImport("user32.dll",SetLastError=true)] static extern IntPtr OpenInputDesktop(uint flags,bool inherit,uint access);
  [DllImport("user32.dll")] static extern bool CloseDesktop(IntPtr desktop);
  [DllImport("user32.dll",CharSet=CharSet.Unicode,SetLastError=true)] static extern bool GetUserObjectInformationW(IntPtr handle,int index,out int value,int length,out int needed);
  [DllImport("wtsapi32.dll",CharSet=CharSet.Unicode)] static extern bool WTSQuerySessionInformationW(IntPtr server,uint session,int info,out IntPtr value,out uint length);
  [DllImport("wtsapi32.dll")] static extern void WTSFreeMemory(IntPtr memory);
  [DllImport("kernel32.dll",CharSet=CharSet.Unicode)] static extern IntPtr LoadLibraryW(string name);
  static readonly JavaScriptSerializer Json=new JavaScriptSerializer();
  readonly Dictionary<string,IntPtr> controls=new Dictionary<string,IntPtr>();
  readonly List<GuardedControl> wrappers=new List<GuardedControl>();
  int enters=0;
  static readonly bool ManualFocus=Environment.GetEnvironmentVariable("NOTYPE_MANUAL_FIXTURE_FOCUS")=="1";
  class GuardedControl:NativeWindow {
    public bool Slow;
    readonly NativeInputFixture owner;
    public GuardedControl(IntPtr handle,NativeInputFixture form){owner=form;AssignHandle(handle);}
    protected override void WndProc(ref Message m){
      if(m.Msg==0x100 && m.WParam.ToInt32()==13)owner.enters++;
      base.WndProc(ref m);
      if(m.Msg==0xc2 && Slow)Thread.Sleep(500);
    }
  }
  NativeInputFixture(){
    Text="notype OWNED input fixture";Width=600;Height=350;ShowInTaskbar=ManualFocus;
    LoadLibraryW("Msftedit.dll");
    int row=0;
    foreach(string id in new[]{"first","second","rich","password","readonly"}){
      int style=unchecked((int)0x50810084);
      if(id=="password")style|=0x20;
      if(id=="readonly")style|=0x800;
      controls[id]=CreateWindowExW(0,id=="rich"?"RICHEDIT50W":"EDIT","alpha beta gamma",style,15,15+row++*50,550,40,Handle,new IntPtr(row+100),IntPtr.Zero,IntPtr.Zero);
      wrappers.Add(new GuardedControl(controls[id],this));
    }
    var instruction=new Label();instruction.Text="Synthetic test only. Click the TOP text box and keep this window active for 20 seconds.";instruction.Left=15;instruction.Top=275;instruction.Width=550;instruction.Height=35;Controls.Add(instruction);
    Shown+=delegate {
      FocusOwned("first");
      Print(new {ready=true,pid=Process.GetCurrentProcess().Id,focusHandle=controls["first"].ToInt64()});
      var reader=new Thread(delegate(){
        string line;
        while((line=Console.ReadLine())!=null){
          var request=Json.Deserialize<Dictionary<string,object>>(line);
          BeginInvoke((Action)delegate {try{Command(request);}catch(Exception error){Print(new {error=error.GetType().Name});}});
        }
        BeginInvoke((Action)delegate{Close();});
      });reader.IsBackground=true;reader.Start();
    };
  }
  void FocusOwned(string id){
    ShowWindow(Handle,5);
    if(!ManualFocus || GetForegroundWindow()==Handle){BringWindowToTop(Handle);Activate();SetForegroundWindow(Handle);}
    SetFocus(id=="unsupported"?Handle:controls[id]);
  }
  void Command(Dictionary<string,object> request){
    string command=(string)request["command"];
    string id=request.ContainsKey("id")?(string)request["id"]:"first";
    IntPtr control=controls.ContainsKey(id)?controls[id]:IntPtr.Zero;
    if(command=="close"){Print(new {closed=true});Close();return;}
    if(command=="focus")FocusOwned(id);
    if(command=="select")SendMessageW(control,0xb1,new IntPtr(Convert.ToInt32(request["start"])),new IntPtr(Convert.ToInt32(request["end"])));
    if(command=="readonly")SendMessageW(control,0xcf,new IntPtr((bool)request["value"]?1:0),IntPtr.Zero);
    if(command=="limit")SendMessageW(control,0xc5,new IntPtr(Convert.ToInt32(request["value"])),IntPtr.Zero);
    if(command=="slow")wrappers[0].Slow=(bool)request["value"];
    if(command=="reset"){
      wrappers[0].Slow=false;
      foreach(var pair in controls){
        if(pair.Key=="readonly")continue;
        SendMessageW(pair.Value,0xcf,IntPtr.Zero,IntPtr.Zero);
        SendMessageW(pair.Value,0xc5,new IntPtr(32767),IntPtr.Zero);
        SetText(pair.Value,0xc,IntPtr.Zero,"alpha beta gamma");
        SendMessageW(pair.Value,0xb1,new IntPtr(6),new IntPtr(10));
      }
      FocusOwned("first");
    }
    var states=new Dictionary<string,object>();
    foreach(var pair in controls){
      var text=new StringBuilder(4096);GetWindowTextW(pair.Value,text,text.Capacity);
      uint start,end;Selection(pair.Value,0xb0,out start,out end);
      states[pair.Key]=new {text=text.ToString(),start=start,end=end};
    }
    int receiving,needed;
    bool queried=GetUserObjectInformationW(GetThreadDesktop(GetCurrentThreadId()),6,out receiving,4,out needed);
    IntPtr input=OpenInputDesktop(0,false,1);int inputError=input==IntPtr.Zero?Marshal.GetLastWin32Error():0;
    if(input!=IntPtr.Zero)CloseDesktop(input);
    IntPtr connection;uint length;bool? active=null;
    if(WTSQuerySessionInformationW(IntPtr.Zero,0xffffffff,8,out connection,out length)){try{if(length>=4)active=Marshal.ReadInt32(connection)==0;}finally{WTSFreeMemory(connection);}}
    Print(new {controls=states,enters=enters,visibleOwned=IsWindowVisible(Handle),foregroundPresent=GetForegroundWindow()!=IntPtr.Zero,foregroundOwned=GetForegroundWindow()==Handle,focusFirst=GetFocus()==controls["first"],desktopInputQueried=queried,desktopReceivesInput=queried&&receiving!=0,inputDesktopAccessible=input!=IntPtr.Zero,inputDesktopError=inputError,sessionActive=active});
  }
  static void Print(object value){Console.WriteLine(Json.Serialize(value));Console.Out.Flush();}
  [STAThread] static void Main(string[] args){
    Console.InputEncoding=Encoding.UTF8;Console.OutputEncoding=new UTF8Encoding(false);
    if(args.Length==3 && args[0]=="focus-owned"){
      uint expected=uint.Parse(args[1]),actual;var handle=new IntPtr(long.Parse(args[2]));
      GetWindowThreadProcessId(handle,out actual);
      if(expected!=actual || !String.Equals(Process.GetProcessById((int)actual).MainModule.FileName,Process.GetCurrentProcess().MainModule.FileName,StringComparison.OrdinalIgnoreCase))throw new InvalidOperationException("Not this owned fixture image");
      Exception failure=null;
      var thread=new Thread(delegate(){try{AutomationElement.FromHandle(handle).SetFocus();}catch(Exception error){failure=error;}});
      thread.SetApartmentState(ApartmentState.MTA);thread.Start();thread.Join();
      if(failure!=null)throw failure;
      return;
    }
    if(args.Length!=0)throw new InvalidOperationException("Unsupported fixture command");
    Application.EnableVisualStyles();Application.Run(new NativeInputFixture());
  }
}
