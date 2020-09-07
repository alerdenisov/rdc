#!/bin/bash

cargo doc \
  --no-deps \
  # Dependencies docs 
  hyper=https://docs.rs/hyper/0.13.7/hyper \
  log=https://docs.rs/log/0.4.11/log/ \
  log=https://docs.rs/log/0.4.11/log/ \
