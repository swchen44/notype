# Provider helper workflow registration

The exact [registration-v1 plan](https://github.com/swchen44/notype/issues/25#issuecomment-6409029819)
has [actual GPT-6-Astra APPROVED / zero unresolved](https://github.com/swchen44/notype/issues/25#issuecomment-6409066492).
It is not executed by the source merger: main remains unmodified. Before any
registration or artifact action, freeze integration S with all documentation,
repin both final source-review deltas to S and verify completed successful ALL
Linux plus Windows contracts at that exact S. The fd14 candidate's successful
runs do not substitute for S. No trailing documentation commit may silently
change the admitted source.

The approved workflow SHA256
`1c80d99fe0b5cfef171fc64bdb3a1e6ec146b99325dd766acb866e5a10fcb8c8`
identifies the LF Git blob. A Windows autocrlf checkout has different physical
bytes; copy the raw blob from S for registration and verify the remote blob
against the approved hash, rather than copying a converted working file.

The new Windows workflow runs synthetic contracts on a feature-branch push. It
does not build or upload an executable on push. Its manually selected artifact
job builds an ordinary helper and never launches it or reads a provider key.

The repository default branch is `main`; a new workflow on the integration or
feature branch cannot be assumed registered for `workflow_dispatch`. Before any
default-branch change, the root coordinator must satisfy the approved narrow
promotion plan and its fresh identity/source/resource checks. Its operation is to
copy only the exact independently reviewed workflow file to `main`, preserving
its bytes, then dispatch it on the integration branch while its HEAD equals the
final merged helper SHA `S`, passing `reviewed_source=S`. The workflow requires
both the actual event `GITHUB_SHA` and clean checkout HEAD equal `S`; root must
verify the run's actual head SHA before accepting any artifact. A moving branch
therefore fails closed. GitHub's [dispatch API](https://docs.github.com/en/rest/actions/workflows#create-a-workflow-dispatch-event)
documents a branch/tag reference; a raw SHA is not assumed supported as `ref`.
Source files, dependencies, credentials and application delivery are excluded
from that promotion. No such change is performed by this implementation.
The push trigger deliberately excludes `main`: a workflow-only registration
commit there must not try to test absent helper source. The later explicit
dispatch checks out and tests the exact complete integration SHA instead.

Before artifact dispatch, independently verify Spec0, Standards0, exact-source
ALL CI and Windows contracts. The `reviewed_source` input must equal the clean
checkout and the repository must be private. The compile-time source pin contains
only that Git SHA. A downloaded artifact still requires its service digest,
individual hashes/imports and root readiness receipt, followed by its own no-key
`check`; a CI upload is not live authorization.
