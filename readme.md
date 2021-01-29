# Simple Transaction Manager

A toy project for managing transactions

## Usage

```
> cargo run -- transactions.csv > accounts.csv
```

Errors are writen to `STDERR`.

## Basics

Functionality | Status | Comments
:------------ | :-------------| :-------------
Builds | :heavy_check_mark::heavy_check_mark:  | 
Runs | :heavy_check_mark::heavy_check_mark:  |  
Read correctly formatted data | :heavy_check_mark::heavy_check_mark:  | 
Write data in correct format | :heavy_check_mark::heavy_check_mark:  | Always outputs 4 decimals

## Completeness

Functionality | Status | Comments
:------------ | :-------------| :-------------
Good deposits | :heavy_check_mark::heavy_check_mark:  |
Good withdrawals | :heavy_check_mark::heavy_check_mark:  |
Bad withdrawals | :heavy_check_mark::heavy_check_mark:  | Should report errors for bad withdrawals
Good disputes | :heavy_check_mark::heavy_check_mark:  |
Bad disputes | :heavy_check_mark::heavy_check_mark:  | Should report errors for bad disputes
Good resolutions | :heavy_check_mark::heavy_check_mark:  |
Bad resolutions | :heavy_check_mark::heavy_check_mark:  | Should report errors for bad resolutions
Good chargebacks | :heavy_check_mark::heavy_check_mark:  |
Bad chargebacks | :heavy_check_mark::heavy_check_mark:  | Should report errors for bad chargebacks

## Correctness

While there is a reasonable amount of unit tests, and good type safety, there isn't as much coverage as I would like.

Also some arbitrary choices were made for undescribed cases, like disputes on withdrawals (handled), or mismatching client ids for disputes (assumed ids are always meaningful). Going back I probably would change the latter of those to reject.

Functionality | Status | Comments
:------------ | :-------------| :-------------
Good deposits | :heavy_check_mark::heavy_check_mark:  |
Good withdrawals | :heavy_check_mark::heavy_check_mark:  |
Bad withdrawals | :heavy_check_mark::heavy_check_mark:  | Reports errors for bad withdrawals
Good disputes | :heavy_check_mark: |
Bad disputes | :heavy_check_mark: | Reports errors for bad disputes
Good resolutions | :heavy_check_mark: |
Bad resolutions | :heavy_check_mark: | Reports errors for bad resolutions
Good chargebacks | :heavy_check_mark::heavy_check_mark:  |
Bad chargebacks | :heavy_check_mark: | Reports errors for bad chargebacks

## Safety and Robustness

Feature | Score | Comments
:------------ | :-------------| :-------------
No unsafe | :heavy_check_mark::heavy_check_mark:  |  
Fuzzing | :heavy_check_mark::heavy_check_mark:  | No crashes
Well supported dependencies | :heavy_check_mark::heavy_check_mark:  |  
Good error handling | :heavy_check_mark::heavy_check_mark:  | Lots of clean `Result` usage
Good errors | :heavy_check_mark: | Not the best error text in all place, but reasonable enough
Rejects malicious behavior | | Didn't get to dealing much with this

## Efficiency

I didn't bother with non-local-memory datastorifying clients, since u16 will fit in memory on even a raspberry pi.

Feature | Score | Comments
:------------ | :-------------| :-------------
Streaming support | :heavy_check_mark::heavy_check_mark:  |  
Low memory usage | :heavy_check_mark::heavy_check_mark:  | Currently everything is in memory, but good support for expansion to non-local-memory datastores where prudent.
Good datastructures | :heavy_check_mark::heavy_check_mark:  | Caching and O(1) where possible
Parallelization/Async | | Not done.

## Maintainability
Feature | Score | Comments
:------------ | :-------------| :-------------
Separation of concerns | :heavy_check_mark::heavy_check_mark:  | Relatively good. Decent file split up.
Interfaces where prudent | :heavy_check_mark::heavy_check_mark:  | 
Comments | :heavy_check_mark: | Some, but not as many as I'd like. Needs more documenting return value, especially in `client.rs`
Unit tests | :heavy_check_mark: | As mentioned earlier, more tests are needed.
