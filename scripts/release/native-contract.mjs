export const cppDlls = Object.freeze(['transcribe.dll','ggml.dll','ggml-base.dll',...['x64','sse42','sandybridge','haswell','alderlake','cannonlake','cascadelake','icelake','skylakex'].map(x=>`ggml-cpu-${x}.dll`)]);
export const crtDlls = Object.freeze(['msvcp140.dll','msvcp140_1.dll','vcruntime140.dll','vcruntime140_1.dll']);
export const nativeNames = Object.freeze(['notype.exe',...cppDlls,...crtDlls]);
