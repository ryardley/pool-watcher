Scanner will effectively be a function:


```
viewing_key -> block[] -> event[]
```

This will accept an array of blocks and return an array of events.

It will be delivered and run as a docker image. 

It will be controlled using a mounted socket.

on start it is passed a railgun viewing key which then leaves it in the ready state

it can then be passed an array of blocks and return an array of events

It does the trial decryption and returns events it found within the batch of blocks

Using the viewing key should be behind a service so it is easy to refactor to support TEE environments such as aws Nitro 


