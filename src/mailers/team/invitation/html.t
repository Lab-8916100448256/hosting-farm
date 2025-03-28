<!DOCTYPE html>
<html>
<head>
    <meta charset="utf-8">
    <title>Team Invitation</title>
</head>
<body style="font-family: Arial, sans-serif; line-height: 1.6; color: #333; max-width: 600px; margin: 0 auto; padding: 20px;">
    <div style="background-color: #f8f9fa; padding: 20px; border-radius: 5px; margin-bottom: 20px;">
        <h1 style="color: #0d6efd; margin-top: 0;">Invitation to Join a Team</h1>
    </div>
    
    <p>Hello {{ name }},</p>
    
    <p>You have been invited to join the team <strong>{{ team_name }}</strong>.</p>
    
    <p>To accept this invitation, please click the button below:</p>
    
    <div style="text-align: center; margin: 30px 0;">
        <a href="{{ invitation_url }}" style="background-color: #0d6efd; color: white; padding: 10px 20px; text-decoration: none; border-radius: 5px; font-weight: bold;">Accept Invitation</a>
    </div>
    
    <p>If you received this invitation by mistake, you can simply ignore this email.</p>
    
    <div style="border-top: 1px solid #dee2e6; margin-top: 20px; padding-top: 20px; font-size: 0.9em; color: #6c757d;">
        <p>This is an automated email, please do not reply.</p>
    </div>
</body>
</html> 