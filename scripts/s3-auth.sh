DATA_DIR="./"

if [ -n "$1" ]; then
	DATA_DIR="$1"
fi

if [ -z "$RUST_LOG" ]; then 
    export RUST_LOG=info,s3_server=debug,s3=debug
fi

cargo run --example s3 -- \
    --access-key AKIAIOSFODNN7EXAMPLE \
    --secret-key wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY \
	--fs-root $DATA_DIR
