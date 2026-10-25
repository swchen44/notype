# Application-specific Windows x64 payload role

Unassembled private role. The final package contains the prebuilt notype app, audited regular-byte DLLs/resources, complete pinned Microsoft Fixed WebView2 tree and applicable notices/provenance. It is not a standalone runtime package. No install/build/download scripts are permitted.

Every published file other than package.json and payload.json must appear in the pinned per-file manifest. Preserve official runtime support files, including vendor x86 helper files inside the complete x64 runtime. x64 entry-point architecture does not establish the required CPU instruction baseline.
