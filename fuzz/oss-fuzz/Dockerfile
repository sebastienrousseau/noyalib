# SPDX-FileCopyrightText: 2026 Google LLC
# SPDX-License-Identifier: Apache-2.0
#
# Copyright 2026 Google LLC
#
# Licensed under the Apache License, Version 2.0 (the "License");
# you may not use this file except in compliance with the License.
# You may obtain a copy of the License at
#
#      http://www.apache.org/licenses/LICENSE-2.0
#
# Unless required by applicable law or agreed to in writing, software
# distributed under the License is distributed on an "AS IS" BASIS,
# WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
# See the License for the specific language governing permissions and
# limitations under the License.
#
################################################################################

# Digest-pinned in THIS repo's copy (OpenSSF Scorecard
# Pinned-Dependencies); the copy submitted to google/oss-fuzz stays
# tag-based per their policy: OSS-Fuzz rebuilds base images
# continuously and requires projects to track them. Refresh the digest
# when re-syncing with upstream:
#   docker buildx imagetools inspect gcr.io/oss-fuzz-base/base-builder-rust
FROM gcr.io/oss-fuzz-base/base-builder-rust@sha256:433319ea1a6bbea6c1db1915a5d385021908cc62026d9f0efda2270ef6060de3
RUN git clone --depth 1 https://github.com/sebastienrousseau/noyalib noyalib
WORKDIR noyalib
COPY build.sh $SRC/
