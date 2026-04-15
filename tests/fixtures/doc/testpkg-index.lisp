(in-package :cl-info)
(let (
(deffn-defvr-pairs '(
; CONTENT: (<INDEX TOPIC> . (<FILENAME> <BYTE OFFSET> <LENGTH IN CHARACTERS> <NODE NAME>))
("greeting" . ("testpkg.info" 955 95 "Definitions for testpkg"))
("hello" . ("testpkg.info" 888 66 "Definitions for testpkg"))
))
(section-pairs '(
; CONTENT: (<NODE NAME> . (<FILENAME> <BYTE OFFSET> <LENGTH IN CHARACTERS>))
("Definitions for testpkg" . ("testpkg.info" 836 214))
("Introduction to testpkg" . ("testpkg.info" 568 128))
)))
(load-info-hashtables (maxima::maxima-load-pathname-directory) deffn-defvr-pairs section-pairs))
