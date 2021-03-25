DATA_DIR="./"

if [ -n "$1" ]; then
	DATA_DIR="$1"
fi

RELEASE=""
if [ "$2" == "--release" ]; then
    RELEASE="--release"
fi

cargo run $RELEASE --features binary \
    -- \
    --access-key AKIAIOSFODNN7EXAMPLE \
    --secret-key wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY \
	--fs-root $DATA_DIR
