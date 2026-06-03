## Lanyard
Lanyard is a local Bring Your Own Key (BYOK) manager that stores sensitive data (API Keys, 
Passwords, Cryptographic pairs) securely inside the native OS Keychain.

This project was created simply to store API keys and other sensitive data 
in the OS Keychain for easy access during development.

We trust ourselves with our keys, and y'know, sometimes we need to copy/paste them again. We didn't 
want to store them in a txt file, we don't like having to log into an account on a website 
to *maybe* be able to copy our keys again (a lot of the time, you get to see it once), and by golly, 
we just like dealing with a nice desktop app.

But that got us thinking, "what if we could just add a button to our *actual* project to 
import a key directly from Lanyard itself?". So that's why we made the Lanyard Python SDK (lanyard-py). 

Want to add Lanyard support to your app? Just `pip install lanyard` to add the 0-dependency library 
and streamline the greater BYOK ecosystem with us!

This project is free and open-source. If you'd like to contribute, be our guest! We'd love to have you contribute :)

> The ULIX Team
