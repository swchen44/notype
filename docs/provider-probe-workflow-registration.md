# Provider helper workflow registration

The new Windows workflow runs synthetic contracts on a feature-branch push. It
does not build or upload an executable on push. Its manually selected artifact
job builds an ordinary helper and never launches it or reads a provider key.

The repository default branch is `main`; a new workflow on the integration or
feature branch cannot be assumed registered for `workflow_dispatch`. Before any
default-branch change, the root coordinator must publish a narrow promotion plan
on #25 and obtain the required GPT-6-Astra approval. The proposed operation is to
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
