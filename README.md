**ptunnel** is Rust program that tunnels connections through HTTPS enabled proxy (supporting CONNECT method), thus ptunnel can be used to use any protocol through proxy - for instance IMAP, SMTP, SSH etc.
ptunnel is using asynchronous I/O (via tokio modules) thus is very effective and can scale well.

Typical usage
=============

Typical usage is for instance if you want to access gmail via corporate proxy - 
locally on your machine you run ptunnel as `ptunnel -p your_proxy_host:port 9993:imap.gmail.com:993 5587:smtp.gmail.com.:587` and set your email client to use localhost:9993 as your IMAP server and localhost:5587 as your SMTP server - this approach has one downfall - SSL/TLS - because now SSL certificates will not match the hostname - so you'll need to add security exception to your email client. 
On linux we can do bit better -  use `/etc/hosts` to map hostnames imap.gmail.com and smtp.gmail.com to local interface:
```
127.0.0.1	imap.gmail.com
127.0.0.1	smtp.gmail.com
```

and use different parameters for ptunnel: `ptunnel -p your_proxy_host:port 9993:gmail-imap.l.google.com:993 5587:gmail-smtp-msa.l.google.com.:587` and setup email client to imap.gmail.com:9993 and smtp.gmail.com:5587 - this will make SSL to work without problems.

Mobile users
============
Mobile users may connect to different networks, where some (corporate network) have proxy and others (home, public wifis) do not.  ptunnel is able to handle such situations,  because if it cannot connect proxy, it falls back to direct connetion to remote host. Thus you can easily move between networks and ptunnel will handle it.

Proxy configuration
===================
You can supply proxy host:port argument to ptunnel program as `-p host:port` or you can use standard environment variable `https_proxy`, which is in form of URL http://host:port. 
Program also supports basic authentication with proxy (via `--user` and `--password` program arguments).

Instalation
===========
Clone repository and build with `cargo build --release` (to install cargo and rust follow instructions here https://www.rustup.rs/)

After successful compilation copy binary `target/release/ptunnel` somewhere on your PATH.

