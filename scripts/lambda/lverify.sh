#!/bin/bash
# lverify.sh <worktree-on-box> <label>: check a worktree's own debug build on AWS Lambda.
# Packages the build (stripped) with the build machine's loader and C library, scratch/, langs/,
# examples/ and tests/python; creates a short-lived function lumen-v-<label>; sweeps every scratch
# program but reader-tail/4 on stack8 and microcode7 (CI's scratch rule); deletes the function.
#   GATES=1  also the Python, PHP and Lumen example gates on all six kernels (test-debug.sh's rule).
#   COUNT=1  also every reference file on both kernels (test_set split by class when the build's
#            unittest supports LUMEN_UNITTEST_ONLY), compared with $WORK/base/$BASE_COUNT.
# Settings: LUMEN_BOX (ssh name of the build machine, required), LUMEN_AWS_PROFILE (default
# lumen-lambda), AWS_REGION (default ap-southeast-1), LUMEN_LAMBDA_ROLE (default
# lambda_basic_execution), PYTHON (a python3 with boto3), WORK (default /tmp/lumen-lambda).
# The build needs LUMEN_ROOT support (fix/relocatable-library) so it finds its library in the package.
set -euo pipefail
wt=$1; label=$2; box=${LUMEN_BOX:?set LUMEN_BOX to the build machine ssh name}
here=$(cd "$(dirname "$0")" && pwd); work=${WORK:-/tmp/lumen-lambda}; py=${PYTHON:-python3}; mkdir -p "$work"
region=${AWS_REGION:-ap-southeast-1}
fn="lumen-v-$(echo "$label" | tr -c 'a-zA-Z0-9-_' '-' | cut -c1-50)"; pkg=$work/pkg-$label.zip
a() { aws --profile "${LUMEN_AWS_PROFILE:-lumen-lambda}" --region "$region" "$@"; }
scp -q "$here/handler.py" "$box:/tmp/handler-lv.py"
ssh "$box" "bash -s" "$wt" "$label" <<'R'
set -e
wt=$1; label=$2; d=/tmp/lvpkg-$label
rm -rf $d && mkdir -p $d/sys $d/tests $d/kernels/stack8 $d/kernels/microcode7
# the kernels name their library as <kernel crate>/../../langs, so the crate folders must exist
touch $d/kernels/stack8/.keep $d/kernels/microcode7/.keep
strip -o $d/lumen-lang $wt/target/debug/lumen-lang
for l in /lib/ld-linux-aarch64.so.1 $(ldd $d/lumen-lang | grep -oE "/[^ ]+\.so[^ ]*"); do cp -L $l $d/sys/; done
cp /tmp/handler-lv.py $d/handler.py
cp -r $wt/scratch $wt/langs $wt/examples $d/ && cp -r $wt/tests/python $d/tests/
(cd $d && rm -f /tmp/lvpkg-$label.zip && zip -q -r -9 /tmp/lvpkg-$label.zip .)
(cd $d && ls scratch/*/*.py | grep -v reader-tail/4.py) > /tmp/lvpkg-$label.progs
(cd $d && ls tests/python/*.py | grep -v "/test_set.py$") > /tmp/lvpkg-$label.tests
(cd $d && find examples/lumen examples/python examples/php -type f \( -name "*.lm" -o -name "*.py" -o -name "*.php" \) | sort) > /tmp/lvpkg-$label.gates
rm -rf $d
R
for x in zip progs tests gates; do scp -q "$box:/tmp/lvpkg-$label.$x" "$work/lv-$label.$x"; done
mv "$work/lv-$label.zip" "$pkg"
ssh "$box" "rm -f /tmp/lvpkg-$label.*" < /dev/null
role=$(a iam get-role --role-name "${LUMEN_LAMBDA_ROLE:-lambda_basic_execution}" --query Role.Arn --output text)
a lambda create-function --function-name "$fn" --runtime python3.12 --architectures arm64 --handler handler.handler \
  --role "$role" --memory-size 1769 --timeout 900 --zip-file "fileb://$pkg" --query State --output text > /dev/null
a lambda wait function-active-v2 --function-name "$fn"
trap 'a lambda delete-function --function-name "$fn" >/dev/null 2>&1; a logs delete-log-group --log-group-name "/aws/lambda/$fn" >/dev/null 2>&1; rm -f "$pkg" "$work"/lv-$label.*' EXIT
FUNCTION="$fn" $py "$here/sweep.py" "$work/lv-$label.progs" 3 700
if [ "${COUNT:-0}" = 1 ]; then
  rm -rf "$work/counts/$label" "$work/src-$label"
  FUNCTION="$fn" $py "$here/lcount.py" "$work/lv-$label.tests" "$work/counts/$label"
  unzip -o -q "$pkg" tests/python/test_set.py langs/lib_python/modules/unittest.py -d "$work/src-$label"
  if grep -q LUMEN_UNITTEST_ONLY "$work/src-$label/langs/lib_python/modules/unittest.py"; then
    FUNCTION="$fn" $py "$here/lsplit.py" "$work/src-$label/tests/python" test_set "$work/counts/$label"
  else
    echo "test_set: not split (this build's unittest cannot select classes); run it on the build machine"
  fi
  rm -rf "$work/src-$label"
  [ -n "${BASE_COUNT:-}" ] && python3 "$here/../suite/count.py" "$work/counts/$label" "$work/base/$BASE_COUNT"
fi
if [ "${GATES:-0}" = 1 ]; then
  FUNCTION="$fn" $py "$here/lgate.py" "$work/lv-$label.gates"
fi
